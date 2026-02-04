use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: usize,
    method: String,
    params: Value,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug)]
pub enum JsonRpcMessage {
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
    Unknown(Value),
}

pub struct LspClient {
    stdin: ChildStdin,
    next_id: AtomicUsize,
    pub receiver: Receiver<JsonRpcMessage>,
}

impl LspClient {
    pub fn stdio(path: &Path) -> std::io::Result<(Self, Child)> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let (tx, rx) = channel();

        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            println!("LSP reader thread started");
            loop {
                let mut line = String::new();
                let mut content_length = 0;

                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 {
                        println!("LSP stdout EOF");
                        return;
                    }
                    println!("LSP header: {:?}", line.trim()); 
                    if line == "\r\n" {
                        break;
                    }

                    if line.starts_with("Content-Length: ") {
                        if let Ok(len) = line["Content-Length: ".len()..].trim().parse::<usize>() {
                            content_length = len;
                        }
                    }
                    line.clear();
                }

                if content_length > 0 {
                    let mut buffer = vec![0u8; content_length];
                    if reader.read_exact(&mut buffer).is_ok() {
                        if let Ok(json_str) = String::from_utf8(buffer) {
                            println!("LSP Body: {}", json_str);
                            // Try to parse as Notification first (method present, no id)
                            // Or Response (id present)
                            // Or generic Value
                            if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
                                let msg = if val.get("method").is_some() && val.get("id").is_none() {
                                    if let Ok(notif) = serde_json::from_value(val.clone()) {
                                        JsonRpcMessage::Notification(notif)
                                    } else {
                                        JsonRpcMessage::Unknown(val)
                                    }
                                } else {
                                    if let Ok(resp) = serde_json::from_value(val.clone()) {
                                        JsonRpcMessage::Response(resp)
                                    } else {
                                        JsonRpcMessage::Unknown(val)
                                    }
                                };
                                let _ = tx.send(msg);
                            }
                        }
                    }
                }
            }
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    eprintln!("LSP Stderr: {}", l);
                }
            }
        });

        Ok((
            Self {
                stdin,
                next_id: AtomicUsize::new(1),
                receiver: rx,
            },
            child,
        ))
    }

    pub fn send_request(&mut self, method: &str, params: Value) -> std::io::Result<usize> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&request)?;
        println!("jflsp -> request: {}", json);
        let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        self.stdin.write_all(content.as_bytes())?;
        self.stdin.flush()?;
        Ok(id)
    }

    pub fn send_notification(&mut self, method: &str, params: Value) -> std::io::Result<()> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let json = serde_json::to_string(&request)?;
        println!("jflsp -> notification: {}", json);
        let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
        self.stdin.write_all(content.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }
}
