//! Formatting and filename helpers matching IDM's observed UI text
//! (language template strings 20421-20423, 20575-20579, 20211, 20219).

use std::path::{Path, PathBuf};

pub fn format_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let f = n as f64;
    if f >= GB {
        format!("{:.1} GB", f / GB)
    } else if f >= MB {
        format!("{:.1} MB", f / MB)
    } else if f >= KB {
        format!("{:.1} KB", f / KB)
    } else {
        format!("{} Bytes", n)
    }
}

pub fn format_speed(bps: u64) -> String {
    if bps == 0 {
        return String::new();
    }
    format!("{}/sec", format_bytes(bps))
}

/// IDM formats the remaining time as "X hours Y min", "X min Y sec" or
/// "X sec" (strings 20421-20423).
pub fn format_time_left(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{} hours {} min", h, m)
    } else if m > 0 {
        format!("{} min {} sec", m, s)
    } else {
        format!("{} sec", s)
    }
}

/// "bytes 0-0/12345" -> (start, end, total)
pub fn parse_content_range(v: &str) -> Option<(u64, u64, Option<u64>)> {
    let v = v.trim();
    let rest = v.strip_prefix("bytes")?.trim();
    let (range, total) = rest.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let start = start.trim().parse::<u64>().ok()?;
    let end = end.trim().parse::<u64>().ok()?;
    let total = if total.trim() == "*" {
        None
    } else {
        Some(total.trim().parse::<u64>().ok()?)
    };
    Some((start, end, total))
}

/// Extract a file name from Content-Disposition, supporting RFC 5987.
pub fn filename_from_disposition(header: &str) -> Option<String> {
    // filename*=UTF-8''name
    for part in header.split(';') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("filename*=") {
            let v = v.trim_matches('"');
            let name = if let Some((_, rest)) = v.split_once("''") {
                percent_decode(rest)
            } else {
                v.to_string()
            };
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    for part in header.split(';') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("filename=") {
            let v = v.trim().trim_matches('"');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .to_string()
}

/// Derive a save name from the URL path, like IDM's "Save As" default.
pub fn filename_from_url(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    let last = u.path_segments()?.last()?.to_string();
    let name = percent_decode(&last);
    if name.is_empty() || name == "/" {
        None
    } else {
        Some(sanitize_filename(&name))
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if (c as u32) < 32 => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches('.').to_string();
    if out.is_empty() {
        out = "download".into();
    }
    // Reserved DOS device names.
    let stem = out
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        out = format!("_{}", out);
    }
    // Keep names comfortably under MAX_PATH when combined with a folder.
    if out.len() > 180 {
        let ext = Path::new(&out)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let stem_len = 180usize.saturating_sub(ext.len());
        let stem: String = out.chars().take(stem_len).collect();
        out = format!("{}{}", stem, ext);
    }
    out
}

/// Choose a non-existing path, producing "name (1).ext" style names.
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let p = Path::new(file_name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for i in 1..10_000 {
        let c = dir.join(format!("{} ({}){}", stem, i, ext));
        if !c.exists() {
            return c;
        }
    }
    dir.join(format!("{} ({}){}", stem, chrono::Utc::now().timestamp(), ext))
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn format_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%m/%d/%Y %H:%M").to_string())
        .unwrap_or_default()
}

/// Total/free space on the volume containing `path`.
pub fn free_space(path: &Path) -> Option<u64> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        #[repr(C)]
        struct UlargeInteger {
            low: u32,
            high: u32,
        }
        extern "system" {
            fn GetDiskFreeSpaceExW(
                dir: *const u16,
                free: *mut UlargeInteger,
                total: *mut UlargeInteger,
                total_free: *mut UlargeInteger,
            ) -> i32;
        }
        let mut free = UlargeInteger { low: 0, high: 0 };
        let mut total = UlargeInteger { low: 0, high: 0 };
        let mut total_free = UlargeInteger { low: 0, high: 0 };
        let ok = unsafe {
            GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free)
        };
        if ok != 0 {
            return Some(((total_free.high as u64) << 32) | total_free.low as u64);
        }
    }
    let _ = path;
    None
}
