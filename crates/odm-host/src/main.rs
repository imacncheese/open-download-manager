//! Chrome/Edge/Firefox native messaging host for Open Download Manager.
//!
//! Browser extension -> this process (stdin/stdout, 32-bit LE length frames)
//! -> Open Download Manager over loopback TCP.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const HOST_NAME: &str = "com.opendm.host";

fn appdata() -> PathBuf {
    if let Ok(v) = std::env::var("ODM_HOME") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("OpenDownloadManager")
}

fn read_msg() -> Option<Value> {
    let mut len_buf = [0u8; 4];
    std::io::stdin().read_exact(&mut len_buf).ok()?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > 16 * 1024 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; len];
    std::io::stdin().read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn write_msg(v: &Value) {
    let s = serde_json::to_vec(v).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    let mut out = std::io::stdout();
    let _ = out.write_all(&(s.len() as u32).to_le_bytes());
    let _ = out.write_all(&s);
    let _ = out.flush();
}

fn ipc_info() -> Option<(u16, String)> {
    let text = std::fs::read_to_string(appdata().join("ipc.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let port = v.get("port")?.as_u64()? as u16;
    let token = v.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string();
    Some((port, token))
}

fn send(port: u16, token: &str, msg: &Value) -> Option<Value> {
    use std::io::{BufRead, BufReader};
    use std::net::TcpStream;
    let mut req = msg.clone();
    if let Some(o) = req.as_object_mut() {
        o.insert("token".into(), json!(token));
    }
    let sock = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().ok()?,
        Duration::from_millis(800),
    )
    .ok()?;
    sock.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    let mut w = sock.try_clone().ok()?;
    let line = serde_json::to_string(&req).ok()?;
    w.write_all(line.as_bytes()).ok()?;
    w.write_all(b"\n").ok()?;
    let mut r = BufReader::new(sock);
    let mut out = String::new();
    r.read_line(&mut out).ok()?;
    serde_json::from_str(out.trim()).ok()
}

fn launch_app() {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("OpenDownloadManager.exe"));

        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local)
                .join("Programs")
                .join("Open Download Manager")
                .join("OpenDownloadManager.exe"),
        );
    }
    candidates.push(PathBuf::from(
        r"C:\Program Files\Open Download Manager\OpenDownloadManager.exe",
    ));
    for c in candidates {
        if c.exists() {
            let _ = std::process::Command::new(c).spawn();
            return;
        }
    }
}

fn with_app(msg: &Value) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut launched = false;
    loop {
        if let Some((port, token)) = ipc_info() {
            if let Some(resp) = send(port, &token, msg) {
                return resp;
            }
        }
        if !launched {
            launch_app();
            launched = true;
        }
        if Instant::now() > deadline {
            return json!({"ok": false, "error": "Open Download Manager is not running"});
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn map_message(msg: &Value) -> Value {
    let cmd = msg.get("cmd").and_then(|v| v.as_str()).unwrap_or("add");
    match cmd {
        "ping" => with_app(&json!({"cmd":"ping"})),
        "add" => with_app(&json!({
            "cmd": "add",
            "url": msg.get("url").and_then(|v| v.as_str()).unwrap_or(""),
            "referer": msg.get("referer").and_then(|v| v.as_str()).unwrap_or(""),
            "cookies": msg.get("cookies").cloned().unwrap_or(Value::Null),
            "user_agent": msg.get("userAgent").cloned().unwrap_or(Value::Null),
            "file_name": msg.get("filename").cloned().unwrap_or(Value::Null),
            "save_dir": msg.get("saveDir").cloned().unwrap_or(Value::Null),
            "category": msg.get("category").cloned().unwrap_or(Value::Null),
            "description": msg.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            "post_data": msg.get("postData").cloned().unwrap_or(Value::Null),
            "start": msg.get("start").and_then(|v| v.as_bool()).unwrap_or(true),
        })),
        "add_many" => with_app(&json!({
            "cmd": "add_many",
            "urls": msg.get("urls").cloned().unwrap_or_else(|| json!([])),
            "referer": msg.get("referer").and_then(|v| v.as_str()).unwrap_or(""),
            "cookies": msg.get("cookies").cloned().unwrap_or(Value::Null),
        })),
        "focus" => with_app(&json!({"cmd":"focus"})),
        _ => json!({"ok": false, "error": "unknown command"}),
    }
}

#[cfg(windows)]
fn install() -> anyhow::Result<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;
    let exe = std::env::current_exe()?;
    let dir = appdata().join("native");
    std::fs::create_dir_all(&dir)?;
    let manifest = json!({
        "name": HOST_NAME,
        "description": "Open Download Manager native messaging host",
        "path": exe.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": ["chrome-extension://*/"]
    });
    let chrome_path = dir.join(format!("{HOST_NAME}.chrome.json"));
    std::fs::write(&chrome_path, serde_json::to_vec_pretty(&manifest)?)?;
    let firefox = json!({
        "name": HOST_NAME,
        "description": "Open Download Manager native messaging host",
        "path": exe.to_string_lossy(),
        "type": "stdio",
        "allowed_extensions": ["odm@local"]
    });
    let ff_path = dir.join(format!("{HOST_NAME}.firefox.json"));
    std::fs::write(&ff_path, serde_json::to_vec_pretty(&firefox)?)?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for sub in [
        format!(r"Software\Google\Chrome\NativeMessagingHosts\{HOST_NAME}"),
        format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{HOST_NAME}"),
        format!(r"Software\CocCoc\Browser\NativeMessagingHosts\{HOST_NAME}"),
    ] {
        let (key, _) = hkcu.create_subkey(sub)?;
        key.set_value("", &chrome_path.to_string_lossy().to_string())?;
    }
    let (key, _) = hkcu.create_subkey(format!(
        r"Software\Mozilla\NativeMessagingHosts\{HOST_NAME}"
    ))?;
    key.set_value("", &ff_path.to_string_lossy().to_string())?;

    // Remove the pre-rename host entries so no old identity remains.
    for old in [
        r"Software\Google\Chrome\NativeMessagingHosts\com.idm.rs",
        r"Software\Microsoft\Edge\NativeMessagingHosts\com.idm.rs",
        r"Software\CocCoc\Browser\NativeMessagingHosts\com.idm.rs",
        r"Software\Mozilla\NativeMessagingHosts\com.idm.rs",
        r"Software\DownloadManager\IDMRS",
    ] {
        let _ = hkcu.delete_subkey_all(old);
    }
    for old in ["com.idm.rs.chrome.json", "com.idm.rs.firefox.json"] {
        let _ = std::fs::remove_file(dir.join(old));
    }
    println!("installed: {}", exe.display());
    Ok(())
}

#[cfg(windows)]
fn uninstall() -> anyhow::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for sub in [
        format!(r"Software\Google\Chrome\NativeMessagingHosts\{HOST_NAME}"),
        format!(r"Software\Microsoft\Edge\NativeMessagingHosts\{HOST_NAME}"),
        format!(r"Software\CocCoc\Browser\NativeMessagingHosts\{HOST_NAME}"),
        format!(r"Software\Mozilla\NativeMessagingHosts\{HOST_NAME}"),
    ] {
        let _ = hkcu.delete_subkey_all(sub);
    }
    let dir = appdata().join("native");
    for name in [
        format!("{HOST_NAME}.chrome.json"),
        format!("{HOST_NAME}.firefox.json"),
    ] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let _ = std::fs::remove_dir(&dir);
    println!("removed browser integration");
    Ok(())
}

#[cfg(windows)]
fn ca_data() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Ok(data) = std::env::var("CustomActionData") {
        for part in data.split(';') {
            if let Some((k, v)) = part.split_once('=') {
                map.insert(k.trim().to_ascii_uppercase(), v.trim().to_string());
            }
        }
    }
    map
}

/// Create the per-user Add/Remove Programs entry (the MSI itself runs in a
/// machine context on some systems, so its ARP entry is hidden).
#[cfg(windows)]
fn arp_install() -> anyhow::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let d = ca_data();
    let code = d.get("PRODUCTCODE").cloned().unwrap_or_default();
    let dir = d.get("INSTALLDIR").cloned().unwrap_or_default();
    let version = d.get("VERSION").cloned().unwrap_or_else(|| "1.0.0".into());
    if code.is_empty() {
        return Ok(());
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(format!(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{code}"
    ))?;
    key.set_value("DisplayName", &"Open Download Manager")?;
    key.set_value("DisplayVersion", &version)?;
    key.set_value("Publisher", &"Open Download Manager contributors")?;
    key.set_value("DisplayIcon", &format!("{dir}\\odm.ico"))?;
    key.set_value("InstallLocation", &dir)?;
    key.set_value("UninstallString", &format!("MsiExec.exe /x {code}"))?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    Ok(())
}

#[cfg(windows)]
fn arp_remove() -> anyhow::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let d = ca_data();
    if let Some(code) = d.get("PRODUCTCODE") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(format!(
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{code}"
        ));
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--arp") {
        #[cfg(windows)]
        {
            if let Err(e) = arp_install() {
                eprintln!("arp install failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--unarp") {
        #[cfg(windows)]
        {
            if let Err(e) = arp_remove() {
                eprintln!("arp remove failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--uninstall") {
        #[cfg(windows)]
        {
            if let Err(e) = uninstall() {
                eprintln!("uninstall failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--install" || a == "--register") {
        #[cfg(windows)]
        {
            if let Err(e) = install() {
                eprintln!("install failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    // Chrome keeps the pipe open; respond to each frame.
    while let Some(msg) = read_msg() {
        let resp = map_message(&msg);
        write_msg(&resp);
    }
}
