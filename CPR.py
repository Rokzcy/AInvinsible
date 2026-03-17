import asyncio
import aiohttp
import random
import time
from typing import List, Dict, Optional
import json
from itertools import cycle

class StealthProxyRotator:
    def __init__(self, proxy_list: List[Dict]):
        """
        proxy_list format: [{"ip": "1.2.3.4", "port": 8080, "type": "http|socks5"}, ...]
        """
        self.proxies = proxy_list
        self.proxy_cycle = cycle(proxy_list)
        self.current_proxy = None
        self.rotation_interval = 3.0  # seconds
        self.session: Optional[aiohttp.ClientSession] = None
        self.rotation_task = None
        
    async def __aenter__(self):
        connector = aiohttp.TCPConnector(limit=100, limit_per_host=30, ttl_dns_cache=300)
        timeout = aiohttp.ClientTimeout(total=10, connect=5)
        self.session = aiohttp.ClientSession(connector=connector, timeout=timeout)
        self.rotation_task = asyncio.create_task(self._rotate_proxies())
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.rotation_task:
            self.rotation_task.cancel()
        if self.session:
            await self.session.close()
    
    async def _rotate_proxies(self):
        """Background task rotates proxy every 3 seconds"""
        while True:
            try:
                self.current_proxy = next(self.proxy_cycle)
                await asyncio.sleep(self.rotation_interval)
            except asyncio.CancelledError:
                break
    
    async def request(self, method: str, url: str, **kwargs) -> aiohttp.ClientResponse:
        """Make request through current rotating proxy"""
        if not self.current_proxy:
            raise RuntimeError("Proxy rotator not initialized")
        
        proxy_url = f"{self.current_proxy['type']}://{self.current_proxy['ip']}:{self.current_proxy['port']}"
        
        # Add random delays and headers for stealth
        kwargs.setdefault('headers', {})
        kwargs['headers'].update({
            'User-Agent': self._get_random_ua(),
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'gzip, deflate',
            'Connection': 'keep-alive',
            'Cache-Control': 'no-cache'
        })
        
        # Rotate immediately after request for max anonymity
        old_proxy = self.current_proxy
        resp = await self.session.request(method, url, proxy=proxy_url, **kwargs)
        
        # Force rotation post-request
        for _ in range(random.randint(1, 3)):  # 1-3 extra rotations
            self.current_proxy = next(self.proxy_cycle)
            
        print(f"[{old_proxy['ip']}:{old_proxy['port']}] -> {resp.status} {url}")
        return resp
    
    def _get_random_ua(self) -> str:
        uas = [
            'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'
        ]
        return random.choice(uas)

# Load 100+ proxies from file/API
def load_proxies(proxy_file: str = "proxies.txt") -> List[Dict]:
    proxies = []
    with open(proxy_file, 'r') as f:
        for line in f:
            line = line.strip()
            if ':' in line:
                ip, port = line.split(':', 1)
                proxies.append({"ip": ip, "port": int(port), "type": "http"})
    print(f"Loaded {len(proxies)} proxies")
    return proxies[:150]  # Cap at 150 for performance
