//! Site grabber: breadth-first exploration of a site with IDM's include and
//! exclude filters (template strings 241-271, 1429-1518, 20580-20673).

use crate::config::wildcard_match;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct GrabberConfig {
    pub start_url: String,
    pub save_dir: PathBuf,
    /// 0 = whole site, otherwise number of link levels.
    pub depth: usize,
    pub same_site_only: bool,
    pub include_exts: Vec<String>,
    pub exclude_exts: Vec<String>,
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub max_files: usize,
    pub max_pages: usize,
    pub explore_concurrency: usize,
    pub download_concurrency: usize,
    pub convert_links: bool,
    pub use_link_text: bool,
}

impl Default for GrabberConfig {
    fn default() -> Self {
        Self {
            start_url: String::new(),
            save_dir: PathBuf::from("."),
            depth: 3,
            same_site_only: true,
            include_exts: vec![],
            exclude_exts: vec![],
            include_paths: vec![],
            exclude_paths: vec![],
            max_files: 1000,
            max_pages: 500,
            explore_concurrency: 4,
            download_concurrency: 4,
            convert_links: false,
            use_link_text: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FoundFile {
    pub url: String,
    pub file_name: String,
    pub referer: String,
    pub page: String,
    pub checked: bool,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct GrabberStats {
    pub pages_processed: usize,
    pub files_found: usize,
    pub files_downloaded: usize,
}

#[derive(Clone, Debug, Default)]
pub struct GrabberResult {
    pub files: Vec<FoundFile>,
    pub stats: GrabberStats,
    pub errors: Vec<String>,
}

fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn ext_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            Path::new(u.path())
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
        })
        .unwrap_or_default()
}

fn matches_any(patterns: &[String], value: &str) -> bool {
    patterns.iter().any(|p| wildcard_match(p, value))
}

fn is_page(url: &str) -> bool {
    let u = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return false,
    };
    let p = u.path().to_ascii_lowercase();
    p.is_empty()
        || p.ends_with('/')
        || p.ends_with(".html")
        || p.ends_with(".htm")
        || p.ends_with(".php")
        || p.ends_with(".asp")
        || p.ends_with(".aspx")
        || p.ends_with(".jsp")
        || !p.rsplit('/').next().unwrap_or("").contains('.')
}

fn clean_link(base: &url::Url, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty()
        || href.starts_with('#')
        || href.starts_with("javascript:")
        || href.starts_with("mailto:")
        || href.starts_with("data:")
    {
        return None;
    }
    let joined = base.join(href).ok()?;
    if joined.scheme() != "http" && joined.scheme() != "https" {
        return None;
    }
    let mut s = joined.to_string();
    if let Some(i) = s.find('#') {
        s.truncate(i);
    }
    Some(s)
}

/// Explore the site and return matching files. Call from a worker thread.
pub fn explore(cfg: &GrabberConfig, cancel: Arc<AtomicBool>) -> Result<GrabberResult> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(explore_async(cfg, cancel))
}

pub async fn explore_async(cfg: &GrabberConfig, cancel: Arc<AtomicBool>) -> Result<GrabberResult> {
    let start = url::Url::parse(&cfg.start_url).map_err(|e| anyhow!("Invalid URL entered: {e}"))?;
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) OpenDownloadManager Grabber/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let link_re = Regex::new(r#"(?i)(?:href|src)\s*=\s*["']([^"']+)["']"#)?;
    let css_re = Regex::new(r#"url\(\s*['"]?([^'")]+)['"]?\s*\)"#)?;
    let base = host_of(&cfg.start_url);
    let mut seen_pages: HashSet<String> = HashSet::new();
    let mut seen_files: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut result = GrabberResult::default();
    queue.push_back((cfg.start_url.clone(), 0));
    seen_pages.insert(cfg.start_url.clone());

    while let Some((page, depth)) = queue.pop_front() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if seen_pages.len() > cfg.max_pages || result.files.len() >= cfg.max_files {
            break;
        }
        let resp = match client.get(&page).send().await {
            Ok(r) => r,
            Err(e) => {
                result.errors.push(format!("{page}: {e}"));
                continue;
            }
        };
        if !resp.status().is_success() {
            result.errors.push(format!("{page}: HTTP {}", resp.status()));
            continue;
        }
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if !ct.contains("html") && !ct.is_empty() && !ct.contains("text") {
            continue;
        }
        let body = match resp.text().await {
            Ok(b) => b,
            Err(_) => continue,
        };
        result.stats.pages_processed += 1;
        let page_url = url::Url::parse(&page).unwrap_or(start.clone());
        let mut links = Vec::new();
        for cap in link_re.captures_iter(&body) {
            if let Some(l) = clean_link(&page_url, &cap[1]) {
                links.push(l);
            }
        }
        for cap in css_re.captures_iter(&body) {
            if let Some(l) = clean_link(&page_url, &cap[1]) {
                links.push(l);
            }
        }
        for link in links {
            let h = host_of(&link);
            if cfg.same_site_only && h != base {
                continue;
            }
            if !cfg.include_paths.is_empty() && !matches_any(&cfg.include_paths, &link) {
                continue;
            }
            if matches_any(&cfg.exclude_paths, &link) {
                continue;
            }
            if is_page(&link) {
                let next_depth = depth + 1;
                let within_depth = cfg.depth == 0 || next_depth <= cfg.depth;
                if within_depth && !seen_pages.contains(&link) {
                    seen_pages.insert(link.clone());
                    queue.push_back((link, next_depth));
                }
                continue;
            }
            let ext = ext_of(&link);
            if !cfg.include_exts.is_empty() && !cfg.include_exts.iter().any(|e| e == &ext) {
                continue;
            }
            if cfg.exclude_exts.iter().any(|e| e == &ext) {
                continue;
            }
            if seen_files.insert(link.clone()) {
                let file_name = crate::util::filename_from_url(&link)
                    .unwrap_or_else(|| format!("file{}", result.files.len()));
                result.files.push(FoundFile {
                    url: link,
                    file_name,
                    referer: page.clone(),
                    page: page.clone(),
                    checked: true,
                    size: None,
                });
                result.stats.files_found = result.files.len();
                if result.files.len() >= cfg.max_files {
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Ok(result)
}
