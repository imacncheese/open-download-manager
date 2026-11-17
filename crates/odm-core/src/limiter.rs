//! Global / per-download speed limiter (IDM "Speed Limiter", strings 1708-1711).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Bucket {
    tokens: f64,
    last: Instant,
}

pub struct SpeedLimiter {
    /// KB/s; 0 = unlimited.
    rate_kbps: AtomicU32,
    bucket: Mutex<Bucket>,
}

impl SpeedLimiter {
    pub fn new(kbps: u32) -> Self {
        Self {
            rate_kbps: AtomicU32::new(kbps),
            bucket: Mutex::new(Bucket {
                tokens: 0.0,
                last: Instant::now(),
            }),
        }
    }

    pub fn set_rate(&self, kbps: u32) {
        self.rate_kbps.store(kbps, Ordering::Relaxed);
        if let Ok(mut b) = self.bucket.lock() {
            b.tokens = 0.0;
            b.last = Instant::now();
        }
    }

    pub fn rate(&self) -> u32 {
        self.rate_kbps.load(Ordering::Relaxed)
    }

    fn rate_bytes(&self) -> f64 {
        self.rate_kbps.load(Ordering::Relaxed) as f64 * 1024.0
    }

    /// Wait until `bytes` may be transferred. Returns immediately when
    /// unlimited.
    pub async fn consume(&self, bytes: usize) {
        let rate = self.rate_bytes();
        if rate <= 0.0 {
            return;
        }
        let need = bytes as f64;
        loop {
            let wait = {
                let mut b = match self.bucket.lock() {
                    Ok(b) => b,
                    Err(_) => return,
                };
                let now = Instant::now();
                let elapsed = now.duration_since(b.last).as_secs_f64();
                b.last = now;
                // Burst of one chunk, but never more than ~1 second worth.
                let cap = rate.max(need);
                b.tokens = (b.tokens + elapsed * rate).min(cap);
                if b.tokens >= need {
                    b.tokens -= need;
                    return;
                }
                let missing = need - b.tokens;
                Duration::from_secs_f64((missing / rate).min(0.1))
            };
            tokio::time::sleep(wait).await;
        }
    }
}
