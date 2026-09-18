//! Deterministic loopback HTTP fixture; never contacts Twitch or a credential store.
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};

pub struct Reply {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub delay: Duration,
}
impl Reply {
    pub fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            headers: Vec::new(),
            delay: Duration::ZERO,
        }
    }
    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
    pub fn header(mut self, key: &str, value: impl ToString) -> Self {
        self.headers.push((key.into(), value.to_string()));
        self
    }
}
pub struct Server {
    pub base: String,
    requests: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    pub async fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
        let task = tokio::spawn(async move {
            let mut handlers = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((socket, _)) = accepted else { break; };
                        let replies = replies.clone(); let recorded = recorded.clone();
                        handlers.spawn(async move { serve(socket, recorded, replies).await; });
                    }
                    _ = handlers.join_next(), if !handlers.is_empty() => {},
                }
            }
        });
        Self {
            base,
            requests,
            task,
        }
    }
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
    pub async fn wait_for_requests(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while self.requests.lock().unwrap().len() < count {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
}
async fn serve(
    mut socket: TcpStream,
    requests: Arc<Mutex<Vec<String>>>,
    replies: Arc<Mutex<VecDeque<Reply>>>,
) {
    let mut request = Vec::new();
    loop {
        let mut buffer = [0; 1024];
        let Ok(count) = socket.read(&mut buffer).await else {
            return;
        };
        if count == 0 {
            return;
        }
        request.extend_from_slice(&buffer[..count]);
        assert!(request.len() < 32 * 1024);
        let text = String::from_utf8_lossy(&request);
        if let Some((headers, body)) = text.split_once("\r\n\r\n") {
            let size = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|n| n.parse::<usize>().ok())
                })
                .unwrap_or(0);
            if body.len() >= size {
                break;
            }
        }
    }
    requests
        .lock()
        .unwrap()
        .push(String::from_utf8(request).unwrap());
    let reply = replies
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or_else(|| Reply::json(418, "unexpected request"));
    tokio::time::sleep(reply.delay).await;
    let mut response = format!(
        "HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for (key, value) in reply.headers {
        response.push_str(&format!("{key}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(&reply.body);
    let _ = socket.write_all(response.as_bytes()).await;
}
