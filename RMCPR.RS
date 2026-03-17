use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, Instant};
use rand::prelude::*;
use reqwest::{Client, ClientBuilder, Proxy, header, Method};
use serde::{Deserialize, Serialize};
use futures::StreamExt;
use tracing::{info, debug, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proxy {
    pub ip: String,
    pub port: u16,
    pub proxy_type: ProxyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyType {
    Http,
    Socks5,
}

#[derive(Clone)]
pub struct StealthRotator {
    proxies: Arc<RwLock<Vec<Proxy>>>,
    current_index: Arc<RwLock<usize>>,
    client: Client,
    rotation_interval: Duration,
}

impl StealthRotator {
    pub fn new(proxies: Vec<Proxy>, rotation_interval: Duration) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, 
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".parse().unwrap());
        headers.insert(header::ACCEPT, 
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".parse().unwrap());
        headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.5".parse().unwrap());
        headers.insert(header::CONNECTION, "keep-alive".parse().unwrap());

        let client = ClientBuilder::new()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .connection_verbose(false)
            .build()
            .expect("Failed to build client");

        Self {
            proxies: Arc::new(RwLock::new(proxies)),
            current_index: Arc::new(RwLock::new(0)),
            client,
            rotation_interval,
        }
    }

    pub async fn start_rotation(&self) {
        let proxies = self.proxies.clone();
        let current_index = self.current_index.clone();
        let interval = interval(self.rotation_interval);

        tokio::spawn(async move {
            let mut interval = interval;
            loop {
                interval.tick().await;
                let mut index = current_index.write().await;
                let len = proxies.read().await.len();
                *index = (*index + 1) % len;
                debug!("Rotated to proxy index: {}", *index);
            }
        });
    }

    pub async fn get_current_proxy(&self) -> Proxy {
        let index = *self.current_index.read().await;
        let proxies = self.proxies.read().await;
        let proxy = proxies[index].clone();
        
        // Force extra rotations for max anonymity
        for _ in 0..thread_rng().gen_range(1..=3) {
            let mut idx = self.current_index.write().await;
            *idx = (*idx + 1) % proxies.len();
        }
        
        proxy
    }

    pub async fn request(&self, method: Method, url: &str) -> Result<reqwest::Response, reqwest::Error> {
        let proxy = self.get_current_proxy().await;
        info!("Using proxy {}:{} -> {}", proxy.ip, proxy.port, url);

        let proxy_url = match proxy.proxy_type {
            ProxyType::Http => format!("http://{}:{}", proxy.ip, proxy.port),
            ProxyType::Socks5 => format!("socks5://{}:{}", proxy.ip, proxy.port),
        };

        let client = ClientBuilder::new()
            .proxy(Proxy::all(proxy_url)?)
            .timeout(Duration::from_secs(8))
            .user_agent(self.random_ua())
            .build()?; 

        client.execute(method.clone(), url).await
    }

    fn random_ua(&self) -> String {
        let uas = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ];
        uas.choose(&mut thread_rng()).unwrap().to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Load 100+ proxies
    let proxies = load_proxies("proxies.txt").await?;
    println!("Loaded {} proxies", proxies.len());

    let rotator = StealthRotator::new(proxies, Duration::from_secs(3));
    rotator.start_rotation().await;

    // Distributed scanning example
    let targets = vec!["http://target.com/admin", "http://target.com/login", "http://target.com/api"];
    let handles = futures::future::join_all(
        targets.iter().map(|url| scan_with_rotator(&rotator, url))
    ).await;

    for result in handles {
        match result {
            Ok(status) => println!("Scan complete: {}", status),
            Err(e) => eprintln!("Scan failed: {}", e),
        }
    }

    Ok(())
}

async fn load_proxies(filename: &str) -> Result<Vec<Proxy>, Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string(filename).await?;
    let mut proxies = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 {
            let ip = parts[0].to_string();
            let port: u16 = parts[1].parse()?;
            proxies.push(Proxy {
                ip,
                port,
                proxy_type: ProxyType::Http,
            });
        }
    }

    Ok(proxies)
}

async fn scan_with_rotator(rotator: &StealthRotator, url: &str) -> Result<u16, reqwest::Error> {
    let resp = rotator.request(Method::GET, url).await?;
    info!("{} -> Status: {}", url, resp.status());
    Ok(resp.status().as_u16())
}
