use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

const MAX_REQUEST: u64 = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const CONFIG: &str = "local-api.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    address: String,
    token: String,
}

pub struct Incoming {
    pub request: Value,
    pub reply: Sender<Value>,
}

pub struct Server {
    config_path: PathBuf,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

pub fn start(directory: &Path) -> Result<(Server, Receiver<Incoming>)> {
    let listener = TcpListener::bind("127.0.0.1:0").context("无法启动本机 API")?;
    listener.set_nonblocking(true)?;
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|e| anyhow::anyhow!("无法生成本机 API 密钥：{e}"))?;
    let config = Config {
        address: listener.local_addr()?.to_string(),
        token: random.iter().map(|b| format!("{b:02x}")).collect(),
    };
    let config_path = directory.join(CONFIG);
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut file, &config)?;
    file.as_file().sync_all()?;
    file.persist(&config_path)
        .context("无法写入本机 API 地址")?;
    let (sender, receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let handle = thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = serve(stream, &config, &sender);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50))
                }
                Err(_) => break,
            }
        }
    });
    Ok((
        Server {
            config_path,
            stop,
            handle: Some(handle),
        },
        receiver,
    ))
}

fn serve(mut stream: TcpStream, config: &Config, sender: &Sender<Incoming>) -> Result<()> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    let mut raw = Vec::new();
    (&mut stream).take(MAX_REQUEST + 1).read_to_end(&mut raw)?;
    let result = (|| -> Result<Value> {
        if raw.len() as u64 > MAX_REQUEST {
            bail!("请求超过 1 MiB");
        }
        let envelope: Value = serde_json::from_slice(&raw).context("请求不是有效 JSON")?;
        if envelope.get("token").and_then(Value::as_str) != Some(&config.token) {
            bail!("API 授权失败");
        }
        let request = envelope.get("request").cloned().context("缺少 request")?;
        let (reply, result) = mpsc::channel();
        sender
            .send(Incoming { request, reply })
            .context("窗口已关闭")?;
        result.recv_timeout(TIMEOUT).context("程序处理请求超时")
    })();
    let response = match result {
        Ok(value) => value,
        Err(e) => json!({"ok":false,"error":format!("{e:#}")}),
    };
    serde_json::to_writer(&mut stream, &response)?;
    stream.flush()?;
    Ok(())
}

pub fn call(directory: &Path, request: &Value) -> Result<Value> {
    let config: Config = serde_json::from_slice(
        &fs::read(directory.join(CONFIG)).context("程序未运行或本机 API 尚未启动")?,
    )?;
    let address: SocketAddr = config.address.parse().context("本机 API 地址无效")?;
    if !address.ip().is_loopback() {
        bail!("本机 API 地址不是回环地址");
    }
    let mut stream =
        TcpStream::connect(address).context("无法连接正在运行的程序；地址文件可能已过期")?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    serde_json::to_writer(
        &mut stream,
        &json!({"token":config.token,"request":request}),
    )?;
    stream.shutdown(Shutdown::Write)?;
    let mut raw = Vec::new();
    stream.take(MAX_REQUEST + 1).read_to_end(&mut raw)?;
    if raw.len() as u64 > MAX_REQUEST {
        bail!("响应超过 1 MiB");
    }
    serde_json::from_slice(&raw).context("程序返回了无效响应")
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = fs::remove_file(&self.config_path);
    }
}
