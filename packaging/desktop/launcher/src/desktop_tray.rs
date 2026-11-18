use std::{
    env, fs,
    net::UdpSocket,
    path::{Path, PathBuf},
    process::Child,
    thread,
    time::{Duration, Instant},
};

use super::native_dictation::{NativeDictation, transcribe_to_clipboard};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use uuid::Uuid;
use winit::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
};

#[cfg(target_os = "windows")]
use super::APP_NAME;
use super::open_browser;

enum Signal {
    Menu(MenuEvent),
    Show,
    DictationToggle,
    DictationFinished(Result<String, String>),
}

fn show_file(base: &Path) -> PathBuf {
    base.join("tray-ipc.port")
}

fn token_file(base: &Path) -> PathBuf {
    base.join("tray-ipc.token")
}

pub fn request_dictation(base: &Path) -> Result<(), String> {
    let port = fs::read_to_string(show_file(base)).map_err(|e| e.to_string())?;
    let token = fs::read_to_string(token_file(base)).map_err(|e| e.to_string())?;
    let port: u16 = port
        .trim()
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let socket = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    socket
        .send_to(
            format!("DICTATE:{}", token.trim()).as_bytes(),
            ("127.0.0.1", port),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn request_show(base: &Path) {
    // The only accepted IPC action makes the existing tray visible again.
    if let Ok(port) = fs::read_to_string(show_file(base))
        && let Ok(port) = port.trim().parse::<u16>()
        && let Ok(socket) = UdpSocket::bind("127.0.0.1:0")
    {
        let _ = socket.send_to(b"SHOW", ("127.0.0.1", port));
    }
}

fn autostart_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("HOME unavailable")?;
    #[cfg(target_os = "macos")]
    return Ok(
        PathBuf::from(home).join("Library/LaunchAgents/io.github.GodsQuantum.ScribeWatch.plist")
    );
    #[cfg(not(target_os = "macos"))]
    return Ok(PathBuf::from(home).join(".config/autostart/scribewatch.desktop"));
}

fn autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
            .ok()
            .and_then(|key| key.get_value::<String, _>(APP_NAME).ok())
            .is_some()
    }
    #[cfg(not(target_os = "windows"))]
    {
        autostart_path().is_ok_and(|path| path.exists())
    }
}

fn set_autostart(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(path)
            .map_err(|e| e.to_string())?;
        if enable {
            let exe = env::current_exe().map_err(|e| e.to_string())?;
            key.set_value(APP_NAME, &format!("\"{}\" --no-browser", exe.display()))
                .map_err(|e| e.to_string())?;
        } else {
            let _ = key.delete_value(APP_NAME);
        }
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let target = autostart_path()?;
        if !enable {
            if target.exists() {
                fs::remove_file(target).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        let exe = env::current_exe().map_err(|e| e.to_string())?;
        fs::create_dir_all(target.parent().ok_or("Invalid autostart parent")?)
            .map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        {
            fn escape(value: &str) -> String {
                value
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            }
            let body = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>io.github.GodsQuantum.ScribeWatch</string><key>ProgramArguments</key><array><string>{}</string><string>--no-browser</string></array><key>RunAtLoad</key><true/></dict></plist>\n",
                escape(&exe.to_string_lossy())
            );
            fs::write(target, body).map_err(|e| e.to_string())?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            let escaped = exe
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            let body = format!(
                "[Desktop Entry]\nType=Application\nName=ScribeWatch\nExec=\"{escaped}\" --no-browser\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
            );
            fs::write(target, body).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn application_icon() -> Result<Icon, String> {
    let bitmap = image::load_from_memory(include_bytes!("../../assets/app.png"))
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let (width, height) = bitmap.dimensions();
    Icon::from_rgba(bitmap.into_raw(), width, height).map_err(|e| e.to_string())
}

pub fn run(port: u16, base: PathBuf, mut child: Child) -> Result<(), String> {
    let event_loop = EventLoop::<Signal>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(Signal::Menu(event));
    }));

    let socket = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let ipc_port = socket.local_addr().map_err(|e| e.to_string())?.port();
    fs::write(show_file(&base), ipc_port.to_string()).map_err(|e| e.to_string())?;
    let token = Uuid::new_v4().to_string();
    fs::write(token_file(&base), &token).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(token_file(&base), fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    let sender = event_loop.create_proxy();
    thread::spawn(move || {
        let mut buf = [0u8; 128];
        while let Ok((count, _)) = socket.recv_from(&mut buf) {
            if &buf[..count] == b"SHOW" {
                let _ = sender.send_event(Signal::Show);
            } else if &buf[..count] == format!("DICTATE:{token}").as_bytes() {
                let _ = sender.send_event(Signal::DictationToggle);
            }
        }
    });

    let menu = Menu::new();
    let open = MenuItem::new("Ouvrir ScribeWatch", true, None);
    let dictate = MenuItem::new("Démarrer la dictée", true, None);
    let startup = MenuItem::new("Lancement automatique", true, None);
    let hide = MenuItem::new(
        "Masquer l'icône (jusqu'à la prochaine ouverture)",
        true,
        None,
    );
    let quit = MenuItem::new("Quitter ScribeWatch", true, None);
    menu.append_items(&[&open, &dictate, &startup, &hide, &quit])
        .map_err(|e| e.to_string())?;
    startup.set_text(if autostart_enabled() {
        "Désactiver le lancement automatique"
    } else {
        "Activer le lancement automatique"
    });
    let image = application_icon()?;
    let mut tray: Option<TrayIcon> = None;
    let mut last_health = Instant::now();
    // Different from browser-only Ctrl+Shift+D to avoid duplicate toggles
    // when the browser is focused and the global shortcut is pressed.
    let shortcut = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
    let hotkey_manager = GlobalHotKeyManager::new().ok();
    let hotkey_registered = hotkey_manager
        .as_ref()
        .is_some_and(|manager| manager.register(shortcut).is_ok());
    if hotkey_registered {
        let proxy = event_loop.create_proxy();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.id == shortcut.id() && event.state == HotKeyState::Pressed {
                let _ = proxy.send_event(Signal::DictationToggle);
            }
        }));
    }
    let event_proxy = event_loop.create_proxy();
    let mut dictation: Option<NativeDictation> = None;
    let mut processing = false;
    event_loop
        .run(move |event, target| {
            let _keep_global_shortcut_registered = &hotkey_manager;
            target.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_secs(2),
            ));
            match event {
                Event::NewEvents(StartCause::Init) => {
                    // Required to initialize macOS status item after run loop starts.
                    match TrayIconBuilder::new()
                        .with_icon(image.clone())
                        .with_menu(Box::new(menu.clone()))
                        .with_tooltip("ScribeWatch — actif sur cet ordinateur")
                        .build()
                    {
                        Ok(icon) => tray = Some(icon),
                        Err(e) => {
                            eprintln!("Tray creation failed: {e}");
                            target.exit();
                        }
                    }
                }
                Event::UserEvent(Signal::Show) => {
                    if let Some(icon) = &tray {
                        let _ = icon.set_visible(true);
                    }
                }
                Event::UserEvent(Signal::DictationToggle) => {
                    if processing {
                        eprintln!("ScribeWatch dictation already transcribing");
                    } else if let Some(recording) = dictation.take() {
                        let _ = dictate.set_text("Transcription en cours…");
                        processing = true;
                        match recording.stop() {
                            Ok(audio) => {
                                let sender = event_proxy.clone();
                                thread::spawn(move || {
                                    let result = transcribe_to_clipboard(port, audio);
                                    let _ = sender.send_event(Signal::DictationFinished(result));
                                });
                            }
                            Err(error) => {
                                eprintln!("ScribeWatch dictation: {error}");
                                processing = false;
                                let _ = dictate.set_text("Démarrer la dictée");
                            }
                        }
                    } else {
                        match NativeDictation::start(&base) {
                            Ok(recording) => {
                                dictation = Some(recording);
                                let _ = dictate.set_text("Arrêter la dictée");
                                if let Some(icon) = &tray {
                                    let _ = icon
                                        .set_tooltip(Some("ScribeWatch — dictée micro en cours"));
                                }
                            }
                            Err(error) => {
                                eprintln!("ScribeWatch microphone: {error}");
                            }
                        }
                    }
                }
                Event::UserEvent(Signal::DictationFinished(outcome)) => {
                    processing = false;
                    let _ = dictate.set_text("Démarrer la dictée");
                    match outcome {
                        Ok(message) => {
                            if let Some(icon) = &tray {
                                let _ = icon.set_tooltip(Some(&message));
                            }
                        }
                        Err(error) => {
                            eprintln!("ScribeWatch transcription: {error}");
                            if let Some(icon) = &tray {
                                let _ = icon.set_tooltip(Some(&format!(
                                    "ScribeWatch — {}",
                                    error.chars().take(180).collect::<String>()
                                )));
                            }
                        }
                    }
                }
                Event::UserEvent(Signal::Menu(event)) => {
                    if event.id == open.id() {
                        let _ = open_browser(port);
                    } else if event.id == dictate.id() {
                        let _ = event_proxy.send_event(Signal::DictationToggle);
                    } else if event.id == startup.id() {
                        if let Err(err) = set_autostart(!autostart_enabled()) {
                            eprintln!("Startup preference failed: {err}");
                        }
                        let _ = startup.set_text(if autostart_enabled() {
                            "Désactiver le lancement automatique"
                        } else {
                            "Activer le lancement automatique"
                        });
                    } else if event.id == hide.id() {
                        if let Some(icon) = &tray {
                            let _ = icon.set_visible(false);
                        }
                    } else if event.id == quit.id() {
                        target.exit();
                    }
                }
                Event::AboutToWait if last_health.elapsed() >= Duration::from_secs(2) => {
                    last_health = Instant::now();
                    if let Ok(Some(_)) = child.try_wait() {
                        target.exit();
                    }
                }
                Event::LoopExiting => {
                    if let Some(active) = dictation.take() {
                        if let Err(error) = active.stop() {
                            eprintln!("ScribeWatch dictation recovery: {error}");
                        }
                    }
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(show_file(&base));
                    let _ = fs::remove_file(token_file(&base));
                }
                _ => {}
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
