//! System clipboard providers and process invocation for `+` and `*` registers.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Maximum payload size allowed for clipboard data (4 MiB).
pub const MAX_CLIPBOARD_BYTES: usize = 4 * 1024 * 1024;

/// Target clipboard selection buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum clipboard_target {
    /// System desktop clipboard (`+` register).
    clipboard,
    /// Primary selection buffer (`*` register, e.g. X11 / Wayland).
    primary,
}

/// Discovered OS clipboard provider command kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum provider_kind {
    /// Wayland `wl-copy` / `wl-paste`
    wayland,
    /// X11 `xclip`
    xclip,
    /// X11 `xsel`
    xsel,
    /// macOS `pbcopy` / `pbpaste`
    macos,
    /// Android / Termux `termux-clipboard-set` / `termux-clipboard-get`
    termux,
    /// Windows PowerShell
    windows_powershell,
    /// Windows `clip.exe` (write-only fallback)
    windows_clip,
    /// No supported clipboard provider detected
    none,
}

/// State for caching provider detection and last written multi-selection fragments.
struct clipboard_manager {
    provider: Mutex<Option<provider_kind>>,
    cached_clipboard_joined: Mutex<Option<String>>,
    cached_clipboard_pieces: Mutex<Option<Vec<String>>>,
    cached_primary_joined: Mutex<Option<String>>,
    cached_primary_pieces: Mutex<Option<Vec<String>>>,
}

impl clipboard_manager {
    const fn new() -> Self {
        Self {
            provider: Mutex::new(None),
            cached_clipboard_joined: Mutex::new(None),
            cached_clipboard_pieces: Mutex::new(None),
            cached_primary_joined: Mutex::new(None),
            cached_primary_pieces: Mutex::new(None),
        }
    }
}

static GLOBAL_CLIPBOARD: clipboard_manager = clipboard_manager::new();

/// Detects the available system clipboard provider once per process.
fn detect_provider() -> provider_kind {
    let mut cached = GLOBAL_CLIPBOARD.provider.lock().unwrap();
    if let Some(kind) = *cached {
        return kind;
    }

    let detected = probe_provider();
    *cached = Some(detected);
    detected
}

/// Probes the system for clipboard utilities without spawning a shell.
fn probe_provider() -> provider_kind {
    #[cfg(target_os = "macos")]
    {
        if Command::new("pbcopy")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
        {
            return provider_kind::macos;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg("exit 0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return provider_kind::windows_powershell;
        }
        if Command::new("clip.exe")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
        {
            return provider_kind::windows_clip;
        }
    }

    // Android / Termux check
    if Command::new("termux-clipboard-get")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
    {
        return provider_kind::termux;
    }

    // Wayland check (wl-copy --version)
    if Command::new("wl-copy")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return provider_kind::wayland;
    }

    // X11 xclip check
    if Command::new("xclip")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return provider_kind::xclip;
    }

    // X11 xsel check
    if Command::new("xsel")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        return provider_kind::xsel;
    }

    provider_kind::none
}

/// Returns the name of the active detected clipboard provider.
#[must_use]
pub fn provider_name() -> &'static str {
    match detect_provider() {
        provider_kind::wayland => "wayland (wl-copy/wl-paste)",
        provider_kind::xclip => "x11 (xclip)",
        provider_kind::xsel => "x11 (xsel)",
        provider_kind::macos => "macos (pbcopy/pbpaste)",
        provider_kind::termux => "termux (termux-clipboard)",
        provider_kind::windows_powershell => "windows (powershell)",
        provider_kind::windows_clip => "windows (clip.exe)",
        provider_kind::none => "none (internal only)",
    }
}

/// Reads text from the specified clipboard target buffer.
///
/// Returns multi-selection pieces if the clipboard content matches
/// Ned's last written multi-selection cache, or a single string otherwise.
///
/// # Errors
///
/// Returns an error if the provider is unavailable, unsupported on the current platform,
/// or if process invocation fails.
pub fn read(target: clipboard_target) -> Result<Vec<String>, String> {
    let prov = detect_provider();
    if prov == provider_kind::none {
        return Err("clipboard provider unavailable".to_string());
    }

    if target == clipboard_target::primary
        && matches!(
            prov,
            provider_kind::macos
                | provider_kind::termux
                | provider_kind::windows_powershell
                | provider_kind::windows_clip
        )
    {
        return Err("primary selection not supported on this platform".to_string());
    }

    let mut cmd = match (prov, target) {
        (provider_kind::wayland, clipboard_target::clipboard) => {
            let mut c = Command::new("wl-paste");
            c.arg("--no-newline");
            c
        }
        (provider_kind::wayland, clipboard_target::primary) => {
            let mut c = Command::new("wl-paste");
            c.args(["--primary", "--no-newline"]);
            c
        }
        (provider_kind::xclip, clipboard_target::clipboard) => {
            let mut c = Command::new("xclip");
            c.args(["-selection", "clipboard", "-out"]);
            c
        }
        (provider_kind::xclip, clipboard_target::primary) => {
            let mut c = Command::new("xclip");
            c.args(["-selection", "primary", "-out"]);
            c
        }
        (provider_kind::xsel, clipboard_target::clipboard) => {
            let mut c = Command::new("xsel");
            c.args(["--clipboard", "--output"]);
            c
        }
        (provider_kind::xsel, clipboard_target::primary) => {
            let mut c = Command::new("xsel");
            c.args(["--primary", "--output"]);
            c
        }
        (provider_kind::macos, clipboard_target::clipboard) => Command::new("pbpaste"),
        (provider_kind::termux, clipboard_target::clipboard) => {
            Command::new("termux-clipboard-get")
        }
        (provider_kind::windows_powershell, clipboard_target::clipboard) => {
            let mut c = Command::new("powershell");
            c.args([
                "-NoProfile",
                "-Command",
                "[Console]::OutputEncoding = [Text::Encoding]::UTF8; Get-Clipboard -Raw",
            ]);
            c
        }
        _ => return Err("clipboard provider unavailable".to_string()),
    };

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("clipboard read failed: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture clipboard stdout".to_string())?;

    let mut buf = Vec::new();
    stdout
        .take(MAX_CLIPBOARD_BYTES as u64)
        .read_to_end(&mut buf)
        .map_err(|e| format!("clipboard read error: {e}"))?;

    let _ = child.wait();

    let text = String::from_utf8_lossy(&buf).to_string();

    // Re-use multiple pieces if current clipboard text equals last Ned-written serialization
    let (cached_joined, cached_pieces) = match target {
        clipboard_target::clipboard => (
            GLOBAL_CLIPBOARD
                .cached_clipboard_joined
                .lock()
                .unwrap()
                .clone(),
            GLOBAL_CLIPBOARD
                .cached_clipboard_pieces
                .lock()
                .unwrap()
                .clone(),
        ),
        clipboard_target::primary => (
            GLOBAL_CLIPBOARD
                .cached_primary_joined
                .lock()
                .unwrap()
                .clone(),
            GLOBAL_CLIPBOARD
                .cached_primary_pieces
                .lock()
                .unwrap()
                .clone(),
        ),
    };

    if let (Some(cj), Some(cp)) = (cached_joined, cached_pieces) {
        if cj == text {
            return Ok(cp);
        }
    }

    Ok(vec![text])
}

/// Writes selection pieces to the specified clipboard target buffer.
///
/// # Errors
///
/// Returns an error if the provider is unavailable, payload exceeds 4 MiB,
/// or child process execution fails.
pub fn write(target: clipboard_target, values: &[String]) -> Result<(), String> {
    let prov = detect_provider();
    if prov == provider_kind::none {
        return Err("clipboard provider unavailable".to_string());
    }

    if target == clipboard_target::primary
        && matches!(
            prov,
            provider_kind::macos
                | provider_kind::termux
                | provider_kind::windows_powershell
                | provider_kind::windows_clip
        )
    {
        return Err("primary selection not supported on this platform".to_string());
    }

    let joined = values.join("\n");
    if joined.len() > MAX_CLIPBOARD_BYTES {
        return Err("clipboard content exceeds 4 MiB limit".to_string());
    }

    let mut cmd = match (prov, target) {
        (provider_kind::wayland, clipboard_target::clipboard) => Command::new("wl-copy"),
        (provider_kind::wayland, clipboard_target::primary) => {
            let mut c = Command::new("wl-copy");
            c.arg("--primary");
            c
        }
        (provider_kind::xclip, clipboard_target::clipboard) => {
            let mut c = Command::new("xclip");
            c.args(["-selection", "clipboard"]);
            c
        }
        (provider_kind::xclip, clipboard_target::primary) => {
            let mut c = Command::new("xclip");
            c.args(["-selection", "primary"]);
            c
        }
        (provider_kind::xsel, clipboard_target::clipboard) => {
            let mut c = Command::new("xsel");
            c.args(["--clipboard", "--input"]);
            c
        }
        (provider_kind::xsel, clipboard_target::primary) => {
            let mut c = Command::new("xsel");
            c.args(["--primary", "--input"]);
            c
        }
        (provider_kind::macos, clipboard_target::clipboard) => Command::new("pbcopy"),
        (provider_kind::termux, clipboard_target::clipboard) => {
            Command::new("termux-clipboard-set")
        }
        (provider_kind::windows_powershell, clipboard_target::clipboard) => {
            let mut c = Command::new("powershell");
            c.args([
                "-NoProfile",
                "-Command",
                "[Console]::InputEncoding = [Text::Encoding]::UTF8; Set-Clipboard -Value ([Console]::In.ReadToEnd())",
            ]);
            c
        }
        (provider_kind::windows_clip, clipboard_target::clipboard) => Command::new("clip.exe"),
        _ => return Err("clipboard provider unavailable".to_string()),
    };

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("clipboard write failed: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(joined.as_bytes())
            .map_err(|e| format!("clipboard write error: {e}"))?;
    }

    let status = child
        .wait()
        .map_err(|e| format!("clipboard wait failed: {e}"))?;
    if !status.success() {
        return Err(format!(
            "clipboard write command exited with error status: {status}"
        ));
    }

    // Cache the written content and pieces for subsequent multi-selection paste matching
    match target {
        clipboard_target::clipboard => {
            *GLOBAL_CLIPBOARD.cached_clipboard_joined.lock().unwrap() = Some(joined);
            *GLOBAL_CLIPBOARD.cached_clipboard_pieces.lock().unwrap() = Some(values.to_vec());
        }
        clipboard_target::primary => {
            *GLOBAL_CLIPBOARD.cached_primary_joined.lock().unwrap() = Some(joined);
            *GLOBAL_CLIPBOARD.cached_primary_pieces.lock().unwrap() = Some(values.to_vec());
        }
    }

    Ok(())
}
