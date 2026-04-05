//! Stdout + optional file log (`tasks/packet-proxy-spec.md`).

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;

#[derive(Clone)]
pub struct PacketLogger {
    inner: Arc<Mutex<LoggerInner>>,
}

struct LoggerInner {
    file: Option<std::fs::File>,
}

impl PacketLogger {
    pub fn new(log_path: Option<&std::path::Path>) -> anyhow::Result<Self> {
        let file = if let Some(p) = log_path {
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("create log directory `{}`", parent.display())
                    })?;
                }
            }
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .with_context(|| format!("open log file `{}`", p.display()))?;
            Some(f)
        } else {
            None
        };
        Ok(Self {
            inner: Arc::new(Mutex::new(LoggerInner { file })),
        })
    }

    pub fn line(&self, s: &str) {
        let mut g = self.inner.lock().expect("logger lock");
        println!("{s}");
        if let Some(ref mut f) = g.file {
            let _ = writeln!(f, "{s}");
            let _ = f.flush();
        }
    }

    pub fn hex_dump(&self, data: &[u8]) {
        for chunk in data.chunks(16) {
            let s: String = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            self.line(&format!("  hex: {s}"));
        }
    }
}

pub fn timestamp_rfc3339_ms() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let ms = d.subsec_millis();
    format!("{secs}.{ms:03}Z")
}
