#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

#[cfg(target_os = "windows")]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW for child CLI/server.
}
#[cfg(not(target_os = "windows"))]
fn hide_console(_: &mut Command) {}

#[cfg(target_os = "windows")]
fn show_error(message: &str) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            flags: u32,
        ) -> i32;
    }
    let text = message
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let caption = "ScribeWatch"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), 0x10);
    }
}

mod desktop_tray;

const APP_NAME: &str = "ScribeWatch";
const APP_ENV: &str = "SCRIBEWATCH";
const SERVER_NAME: &str = "scribewatch-server";
const HEALTH: &str = "ready";
const DEFAULT_PORT: u16 = 3052;

fn resources() -> Result<PathBuf, String> {
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    let parent = executable.parent().ok_or("Missing executable parent")?;
    if cfg!(target_os = "macos") {
        Ok(parent
            .parent()
            .ok_or("Missing .app Contents directory")?
            .join("Resources"))
    } else {
        Ok(parent.to_path_buf())
    }
}

fn data_root() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let path = env::var_os("LOCALAPPDATA")
            .or_else(|| env::var_os("APPDATA"))
            .ok_or("Local application data folder is not configured")?;
        return Ok(PathBuf::from(path).join(APP_NAME));
    }
    #[cfg(target_os = "macos")]
    {
        let home = env::var_os("HOME").ok_or("HOME is not configured")?;
        return Ok(PathBuf::from(home)
            .join("Library/Application Support")
            .join(APP_NAME));
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let home = env::var_os("HOME").ok_or("HOME is not configured")?;
        Ok(PathBuf::from(home).join(".local/share").join(APP_NAME))
    }
}

fn executable(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn check_binary(file: &Path, arg: &str) -> Result<String, String> {
    let mut command = Command::new(file);
    command.arg(arg);
    hide_console(&mut command);
    let result = command
        .output()
        .map_err(|e| format!("{}: {e}", file.display()))?;
    if !result.status.success() {
        return Err(format!("{} exited with {}", file.display(), result.status));
    }
    Ok(String::from_utf8_lossy(&result.stdout).into_owned())
}

fn http_health(port: u16) -> Result<String, String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(800))
        .map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    write!(
        stream,
        "GET /api/v1/{HEALTH} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .map_err(|e| e.to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| e.to_string())?;
    if !response.starts_with("HTTP/1.1 200") && !response.starts_with("HTTP/1.0 200") {
        return Err("The port is occupied by a nonhealthy server".to_owned());
    }
    if !response.contains(&format!("\"version\":\"{}\"", env!("CARGO_PKG_VERSION"))) {
        return Err("The port is occupied by another application/version".to_owned());
    }
    Ok(response)
}

fn open_browser(port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/");
    // Browser app mode uses the OS web renderer without a second Electron/Tauri
    // bundle. A normal browser remains a safe fallback on any unsupported host.
    #[cfg(target_os = "windows")]
    {
        for root in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Some(prefix) = env::var_os(root) {
                for relative in [
                    "Microsoft/Edge/Application/msedge.exe",
                    "Google/Chrome/Application/chrome.exe",
                ] {
                    let exe = PathBuf::from(&prefix).join(relative);
                    if exe.is_file() {
                        let mut command = Command::new(exe);
                        command.arg(format!("--app={url}"));
                        hide_console(&mut command);
                        return command.spawn().map(|_| ()).map_err(|e| e.to_string());
                    }
                }
            }
        }
        let mut fallback = Command::new("explorer.exe");
        fallback.arg(&url);
        hide_console(&mut fallback);
        return fallback.spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[cfg(target_os = "macos")]
    {
        if Path::new("/Applications/Google Chrome.app").exists() {
            return Command::new("open")
                .args(["-a", "Google Chrome", "--args", &format!("--app={url}")])
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        return Command::new("open")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn launch(smoke: bool, no_browser: bool) -> Result<(), String> {
    let root = resources()?;
    let server = root.join("bin").join(executable(SERVER_NAME));
    let ffmpeg = root.join("bin").join(executable("ffmpeg"));
    let ffprobe = root.join("bin").join(executable("ffprobe"));
    let frontend = root.join("frontend");
    if !frontend.join("index.html").is_file() {
        return Err(format!("Bundled frontend missing: {}", frontend.display()));
    }
    let version = check_binary(&server, "--version")?;
    if !version.contains(env!("CARGO_PKG_VERSION")) {
        return Err("Backend version does not match installer".into());
    }
    check_binary(&ffmpeg, "-version")?;
    check_binary(&ffprobe, "-version")?;
    if APP_ENV == "AUTOSUBS" {
        let filters = check_binary(&ffmpeg, "-filters")?;
        if !filters.contains("subtitles") || !filters.contains("ass") {
            return Err("Bundled FFmpeg does not provide libass/subtitles filters".into());
        }
    }
    if env::args().any(|arg| arg == "--check-install") {
        println!(
            "DESKTOP_BUNDLE_VALID {} {}",
            APP_NAME,
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    let base = data_root()?;
    let config = base.join("config");
    let data = base.join("data");
    let fonts = data.join("fonts");
    for folder in [&base, &config, &data, &fonts] {
        fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    let port = env::var(format!("{APP_ENV}_PORT"))
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    if http_health(port).is_ok() {
        if smoke {
            return Err("A server already occupies the smoke test port".into());
        }
        // A second launch restores the hidden tray without restarting the backend.
        desktop_tray::request_show(&base);
        if !no_browser {
            open_browser(port)?;
        }
        return Ok(());
    }
    if TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).is_ok() {
        return Err(format!("Port {port} is used by another application"));
    }

    let path = env::var_os("PATH").unwrap_or_default();
    let path = env::join_paths(std::iter::once(root.join("bin")).chain(env::split_paths(&path)))
        .map_err(|e| format!("PATH: {e}"))?;
    let userhome = env::var_os(if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    })
    .ok_or("Unable to determine user home")?;
    let roots = env::join_paths([PathBuf::from(userhome), data.clone()])
        .map_err(|e| format!("Allowed roots: {e}"))?;
    let logpath = base.join("desktop.log");
    let log: File = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&logpath)
        .map_err(|e| e.to_string())?;
    let mut command = Command::new(&server);
    hide_console(&mut command);
    let mut child = command
        .env(format!("{APP_ENV}_HOST"), "127.0.0.1")
        .env(format!("{APP_ENV}_PORT"), port.to_string())
        .env(format!("{APP_ENV}_CONFIG_DIR"), &config)
        .env(format!("{APP_ENV}_DATA_DIR"), &data)
        .env(format!("{APP_ENV}_DIST_DIR"), &frontend)
        .env(format!("{APP_ENV}_ALLOWED_ROOTS"), roots)
        .env("PATH", path)
        .current_dir(&root)
        .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|e| format!("Start native server: {e}"))?;
    let ready = (0..100).any(|_| {
        if http_health(port).is_ok() {
            return true;
        }
        if let Ok(Some(_)) = child.try_wait() {
            return false;
        }
        thread::sleep(Duration::from_millis(250));
        false
    });
    if !ready {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "Server did not become healthy; see {}",
            logpath.display()
        ));
    }
    if smoke {
        child.kill().map_err(|e| e.to_string())?;
        child.wait().map_err(|e| e.to_string())?;
        println!(
            "DESKTOP_SMOKE_OK {} {}",
            APP_NAME,
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if !no_browser {
        open_browser(port)?;
    }
    // Existing CLI probes still exit without any GUI, including CI --smoke.
    // In GUI mode a native tray owns the server lifetime; no extra service.
    if !env::args().any(|arg| arg == "--no-tray")
        && (cfg!(target_os = "windows")
            || cfg!(target_os = "macos")
            || env::var_os("DISPLAY").is_some()
            || env::var_os("WAYLAND_DISPLAY").is_some())
    {
        return desktop_tray::run(port, base, child);
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("Server exited: {status}"));
    }
    Ok(())
}

fn main() {
    let smoke = env::args().any(|arg| arg == "--smoke");
    let no_browser = env::args().any(|arg| arg == "--no-browser");
    if let Err(error) = launch(smoke, no_browser) {
        eprintln!("{APP_NAME}: {error}");
        #[cfg(target_os = "windows")]
        show_error(&error);
        std::process::exit(1);
    }
}
