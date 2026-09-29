use std::sync::{Arc, RwLock};
use std::time::Duration;

use content::Entry;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Loading,
    Ready,
    Offline,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub version: u64,
    pub status: Status,
    pub entries: Vec<Entry>,
}

pub type Shared = Arc<RwLock<Snapshot>>;

pub fn shared() -> Shared {
    Arc::new(RwLock::new(Snapshot {
        version: 0,
        status: Status::Loading,
        entries: Vec::new(),
    }))
}

pub fn spawn_refresher(shared: Shared, address: String) {
    tokio::spawn(async move {
        loop {
            let result = tokio::time::timeout(Duration::from_secs(5), fetch(&address)).await;
            {
                let mut snapshot = shared
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                match result {
                    Ok(Ok(entries)) => {
                        if snapshot.status != Status::Ready || snapshot.entries != entries {
                            snapshot.entries = entries;
                            snapshot.status = Status::Ready;
                            snapshot.version += 1;
                        }
                    }
                    _ => {
                        if snapshot.status != Status::Offline {
                            snapshot.status = Status::Offline;
                            snapshot.version += 1;
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

async fn fetch(address: &str) -> Result<Vec<Entry>, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = TcpStream::connect(address).await?;
    let request =
        format!("GET /entries HTTP/1.0\r\nHost: {address}\r\nAccept: application/json\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut response = Vec::new();
    stream
        .take(4 * 1024 * 1024)
        .read_to_end(&mut response)
        .await?;
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("malformed response")?;
    let head = std::str::from_utf8(&response[..split])?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        return Err("unexpected status".into());
    }
    Ok(serde_json::from_slice(&response[split + 4..])?)
}
