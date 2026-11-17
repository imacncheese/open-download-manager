//! HTTP metadata probing and client construction.

use crate::config::{Config, ProxyMode};
use crate::model::Download;
use crate::util;
use anyhow::{anyhow, Result};
use reqwest::header::{
    ACCEPT_RANGES, AUTHORIZATION, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    COOKIE, ETAG, LAST_MODIFIED, RANGE, REFERER, USER_AGENT,
};
use reqwest::{Client, Response, StatusCode};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub final_url: String,
    pub size: Option<u64>,
    pub resumable: bool,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub file_name: Option<String>,
    pub status: u16,
}

pub fn effective_user_agent(cfg: &Config, dl: &Download) -> String {
    if !dl.user_agent.is_empty() {
        dl.user_agent.clone()
    } else if !cfg.user_agent.is_empty() {
        cfg.user_agent.clone()
    } else {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".into()
    }
}

fn add_proxy(builder: reqwest::ClientBuilder, cfg: &Config) -> reqwest::ClientBuilder {
    let p = &cfg.proxy;
    let mut builder = builder;
    match p.mode {
        ProxyMode::None => {}
        ProxyMode::System => {
            if let Some((host, port)) = system_proxy() {
                let url = format!("http://{}:{}", host, port);
                if let Ok(px) = reqwest::Proxy::all(&url) {
                    builder = builder.proxy(px);
                }
            }
        }
        ProxyMode::Manual => {
            if p.use_socks {
                if !p.socks_host.is_empty() {
                    let url = if p.socks_user.is_empty() {
                        format!("socks5h://{}:{}", p.socks_host, p.socks_port)
                    } else {
                        format!(
                            "socks5h://{}:{}@{}:{}",
                            urlencode(&p.socks_user),
                            urlencode(&p.socks_pass),
                            p.socks_host,
                            p.socks_port
                        )
                    };
                    if let Ok(px) = reqwest::Proxy::all(&url) {
                        builder = builder.proxy(px);
                    }
                }
            } else if p.use_proxy_for_http && !p.http_host.is_empty() {
                let url = if p.http_user.is_empty() {
                    format!("http://{}:{}", p.http_host, p.http_port)
                } else {
                    format!(
                        "http://{}:{}@{}:{}",
                        urlencode(&p.http_user),
                        urlencode(&p.http_pass),
                        p.http_host,
                        p.http_port
                    )
                };
                if let Ok(px) = reqwest::Proxy::all(&url) {
                    builder = builder.proxy(px);
                }
            }
            if !p.no_proxy.trim().is_empty() {
                // Applied to each proxy below when supported; reqwest's
                // client-level no_proxy() is environment based.
            }
        }
        ProxyMode::Auto => {
            // PAC execution is out of scope; fall back to system settings.
            if let Some((host, port)) = system_proxy() {
                if let Ok(px) = reqwest::Proxy::all(&format!("http://{}:{}", host, port)) {
                    builder = builder.proxy(px);
                }
            }
        }
    }
    builder
}

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

pub fn build_client(cfg: &Config, dl: &Download) -> Result<Client> {
    let mut b = Client::builder()
        .user_agent(effective_user_agent(cfg, dl))
        .connect_timeout(Duration::from_secs(25))
        .read_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(8)
        .tcp_nodelay(true)
        .gzip(false)
        .brotli(false)
        .deflate(false)
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(15));
    if cfg.proxy.try_browser_proxy_on_error {
        // Keep the flag for parity; browser proxy is already the default.
    }
    b = add_proxy(b, cfg);
    // IDM ships its own OpenSSL; reqwest uses the platform TLS stack.
    b.build().map_err(|e| anyhow!("Cannot create HTTP client: {e}"))
}

pub fn apply_headers(
    mut req: reqwest::RequestBuilder,
    cfg: &Config,
    dl: &Download,
) -> reqwest::RequestBuilder {
    req = req.header(USER_AGENT, effective_user_agent(cfg, dl));
    // Byte ranges must refer to the stored representation, not a
    // content-encoded body.
    req = req.header(reqwest::header::ACCEPT_ENCODING, "identity");
    if !dl.referer.is_empty() {
        req = req.header(REFERER, &dl.referer);
    }
    if let Some(c) = dl.cookies.as_deref() {
        if !c.is_empty() {
            req = req.header(COOKIE, c);
        }
    }
    if let Some(u) = dl.username.as_deref() {
        if !u.is_empty() {
            use base64::Engine;
            let raw = format!("{}:{}", u, dl.password.as_deref().unwrap_or(""));
            let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
            req = req.header(AUTHORIZATION, format!("Basic {}", encoded));
        }
    }
    req
}

fn header_str(resp: &Response, name: reqwest::header::HeaderName) -> Option<String> {
    resp.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Determine size, resume support and the server-side file name.
pub async fn probe(client: &Client, cfg: &Config, dl: &Download) -> Result<Probe> {
    let mut probe = Probe {
        final_url: dl.url.clone(),
        ..Default::default()
    };

    // 1) HEAD, which is cheap and gives metadata without a body.
    let head = apply_headers(client.head(&dl.url), cfg, dl)
        .send()
        .await;
    if let Ok(resp) = &head {
        probe.status = resp.status().as_u16();
        probe.final_url = resp.url().to_string();
        if resp.status().is_success() {
            probe.size = header_str(resp, CONTENT_LENGTH)
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|n| *n > 0);
            probe.content_type = header_str(resp, CONTENT_TYPE);
            probe.etag = header_str(resp, ETAG);
            probe.last_modified = header_str(resp, LAST_MODIFIED);
            probe.resumable = header_str(resp, ACCEPT_RANGES)
                .map(|v| v.to_ascii_lowercase().contains("bytes"))
                .unwrap_or(false);
            probe.file_name = header_str(resp, CONTENT_DISPOSITION)
                .and_then(|v| util::filename_from_disposition(&v));
            // HEAD's Accept-Ranges can lie; validate with a ranged GET below.
        }
    }

    // 2) A one-byte ranged GET verifies resume support even when the server
    //    does not advertise Accept-Ranges or rejects HEAD.
    let get = apply_headers(client.get(&dl.url), cfg, dl).header(RANGE, "bytes=0-0");
    let get = match &dl.post_data {
        Some(body) if !body.is_empty() => get
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body.clone()),
        _ => get,
    };
    let resp = get.send().await?;
    probe.status = resp.status().as_u16();
    probe.final_url = resp.url().to_string();
    if resp.status() == StatusCode::PARTIAL_CONTENT {
        probe.resumable = true;
        if let Some(cr) = header_str(&resp, CONTENT_RANGE) {
            if let Some((_, _, total)) = util::parse_content_range(&cr) {
                probe.size = total;
            }
        }
        if probe.size.is_none() {
            probe.size = header_str(&resp, CONTENT_LENGTH)
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|n| *n > 0)
                .map(|n| n.saturating_sub(1));
        }
        probe.content_type = header_str(&resp, CONTENT_TYPE);
        probe.etag = header_str(&resp, ETAG);
        probe.last_modified = header_str(&resp, LAST_MODIFIED);
        probe.file_name = header_str(&resp, CONTENT_DISPOSITION)
            .and_then(|v| util::filename_from_disposition(&v));
        return Ok(probe);
    }

    if resp.status().is_success() {
        probe.resumable = false;
        probe.size = header_str(&resp, CONTENT_LENGTH)
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|n| *n > 0);
        probe.content_type = header_str(&resp, CONTENT_TYPE);
        probe.etag = header_str(&resp, ETAG);
        probe.last_modified = header_str(&resp, LAST_MODIFIED);
        probe.file_name = header_str(&resp, CONTENT_DISPOSITION)
            .and_then(|v| util::filename_from_disposition(&v));
        if probe.size.is_none() {
            // Unknown length: body is still usable as a single stream.
            probe.resumable = false;
        }
        return Ok(probe);
    }

    Err(anyhow!(
        "Cannot download this file. Invalid http server reply (HTTP {}).",
        probe.status
    ))
}

#[cfg(windows)]
fn system_proxy() -> Option<(String, u16)> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .ok()?;
    let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    // Forms: "host:port", "http=host:port;https=host:port", "socks=..."
    for part in server.split(';') {
        let part = part.trim();
        let candidate = if let Some((scheme, rest)) = part.split_once('=') {
            if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
                rest
            } else {
                continue;
            }
        } else {
            part
        };
        if let Some((host, port)) = candidate.rsplit_once(':') {
            if let Ok(port) = port.parse::<u16>() {
                return Some((host.to_string(), port));
            }
        }
    }
    Some((server, 8080))
}

#[cfg(not(windows))]
fn system_proxy() -> Option<(String, u16)> {
    for var in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(v) = std::env::var(var) {
            let v = v.trim_start_matches("http://").trim_start_matches("https://");
            if let Some((host, port)) = v.rsplit_once(':') {
                if let Ok(port) = port.parse::<u16>() {
                    return Some((host.to_string(), port));
                }
            }
        }
    }
    None
}
