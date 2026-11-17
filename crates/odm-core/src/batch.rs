//! Batch download URL expansion ("Batch download", template strings 280,
//! 1553-1560, 20645-20648) and link import helpers.

use regex::Regex;

pub fn has_wildcard(url: &str) -> bool {
    url.contains('*')
}

/// Expand `http://host/img*.jpg` into a list using the given numeric range.
/// `width` zero-pads the replacement like IDM's "Wildcard size".
pub fn expand_range(url: &str, from: u64, to: u64, width: usize) -> Vec<String> {
    if !has_wildcard(url) || to < from {
        return vec![url.to_string()];
    }
    let mut out = Vec::new();
    for n in from..=to {
        let repl = if width > 0 {
            format!("{:0width$}", n, width = width)
        } else {
            n.to_string()
        };
        out.push(url.replacen('*', &repl, 1));
    }
    out
}

/// Letter ranges, used by IDM for `file*.zip` when "Letters" is selected.
pub fn expand_letters(url: &str, from: char, to: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut c = from;
    while c <= to {
        out.push(url.replacen('*', &c.to_string(), 1));
        c = char::from_u32(c as u32 + 1).unwrap_or(to);
        if c == '\0' {
            break;
        }
    }
    out
}

/// Find all http(s) links in arbitrary text (clipboard, text file import).
pub fn parse_links(text: &str) -> Vec<String> {
    let re = Regex::new(r#"https?://[^\s"'<>\[\]{}|\\^`]+"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for m in re.find_iter(text) {
        let mut s = m.as_str().trim_end_matches(['.', ',', ')', ';', ':']).to_string();
        while s.ends_with(')') && s.matches('(').count() < s.matches(')').count() {
            s.pop();
        }
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

/// Parse an IDM text export: one URL per line, optional tab-separated fields.
pub fn parse_export(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split('\t');
        let url = it.next().unwrap_or("").trim().to_string();
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ftp://") {
            let desc = it.next().unwrap_or("").trim().to_string();
            out.push((url, desc));
        }
    }
    out
}

pub fn export_text(items: &[(String, String)]) -> String {
    let mut s = String::from("# Open Download Manager export file\r\n");
    for (url, desc) in items {
        s.push_str(url);
        if !desc.is_empty() {
            s.push('\t');
            s.push_str(desc);
        }
        s.push_str("\r\n");
    }
    s
}
