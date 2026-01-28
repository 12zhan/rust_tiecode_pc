use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: usize,
    method: String,
    params: Value,
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<usize>,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub method: Option<String>,
    pub params: Option<Value>,
}

pub struct LspClient {
    stdin: ChildStdin,
    next_id: AtomicUsize,
}

impl LspClient {
    pub fn new<F>(cmd: &str, args: &[&str], on_message: F) -> std::io::Result<Self>
    where
        F: Fn(JsonRpcResponse) + Send + 'static,
    {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                let mut content_length = 0;

                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 { return; }
                    if line == "\r\n" { break; }
                    
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
                            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&json_str) {
                                on_message(response);
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

        Ok(Self {
            stdin,
            next_id: AtomicUsize::new(1),
        })
    }

    pub fn send_request(&mut self, method: &str, params: Value) -> usize {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        if let Ok(json) = serde_json::to_string(&request) {
            let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
            let _ = self.stdin.write_all(content.as_bytes());
        }
        id
    }
    
    pub fn send_notification(&mut self, method: &str, params: Value) {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        if let Ok(json) = serde_json::to_string(&request) {
            let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
            let _ = self.stdin.write_all(content.as_bytes());
        }
    }
}
