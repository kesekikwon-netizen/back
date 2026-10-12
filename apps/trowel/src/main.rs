//! Trowel's host binary: the eframe window that carries [`VectorcraftApp`].
//!
//! Trowel reuses the whole VectorCraft engine, renderer, tools and panel set.
//! The only thing this app draws itself is the window arrangement — design 2.1
//! section 3 — because `VectorcraftApp::ui_frame` is private and
//! [`shell::trowel_frame`] replaces it (J003).
//!
//! Everything else here is host plumbing — file dialogs, preferences, the
//! control channel — and follows `apps/vectorcraft/src/main.rs`.
//!
//! Rules that bind this file (see `AGENTS.md`): no `unwrap`/`expect`/`panic`
//! outside tests, no implicit panics, and user-visible strings in English.

mod control_server;
mod logging;
mod shell;
mod window;

use vectorcraft_engine::Session;
use vectorcraft_engine::cmd::fileio;
use vectorcraft_engine::file_access;
use vectorcraft_ui_egui::picks::{PickAnswer, PickRequest};
use vectorcraft_ui_egui::{FilePick, Services, VectorcraftApp};

/// Where UI preferences live. Trowel keeps its own folder, so its settings never
/// mix with the VectorCraft settings on the same machine: `%APPDATA%\Trowel`
/// (Windows), `$XDG_CONFIG_HOME/trowel` or `~/.config/trowel` (Linux and macOS).
fn prefs_path() -> Option<std::path::PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(std::path::PathBuf::from)?
    } else {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(d) => std::path::PathBuf::from(d),
            None => std::path::PathBuf::from(std::env::var_os("HOME")?).join(".config"),
        }
    };
    Some(base.join("trowel").join("trowel-ui.json"))
}

/// Where the log files live: `logs` in the preferences folder.
fn log_dir() -> Option<std::path::PathBuf> {
    Some(prefs_path()?.parent()?.join("logs"))
}

/// Runs without preferences (`TROWEL_NO_PREFS`, agents' test runs) neither reads
/// nor writes them, and leaves no log file behind.
fn prefs_enabled() -> bool {
    std::env::var_os("TROWEL_NO_PREFS").is_none()
}

/// The saved UI preferences, read before the window opens (they hold its size and
/// position). A file that can't be read or parsed leaves the defaults.
fn read_prefs() -> Option<vectorcraft_ui_egui::UiState> {
    if !prefs_enabled() {
        return None;
    }
    let bytes = prefs_path().and_then(|p| std::fs::read(p).ok())?;
    serde_json::from_slice(&bytes).ok()
}

fn load_prefs(app: &mut VectorcraftApp, saved: Option<vectorcraft_ui_egui::UiState>) {
    if !prefs_enabled() {
        return;
    }
    if let Some(ui) = saved {
        app.ui = ui.sanitized();
    }
    vectorcraft_ui_egui::prefs_dialog::restore(app);
}

fn save_prefs(app: &VectorcraftApp) {
    if !prefs_enabled() {
        return;
    }
    let Some(path) = prefs_path() else { return };
    let mut ui = app.ui.clone();
    ui.engine_prefs = app.session.prefs.to_json();
    // Preferences are best effort: a failed write keeps the previous file, and
    // the app is already closing, so there is nothing left to tell the user.
    if let Ok(bytes) = serde_json::to_vec_pretty(&ui) {
        let _ = fileio::write_atomic(&path, &bytes);
    }
}

/// The window the file dialogs belong to.
type Parent = Option<std::sync::Arc<winit::window::Window>>;

/// A native file dialog for `request` over `parent` (Windows and Linux, where the
/// portal then shows it over the window and keeps it in front; macOS shows it as before).
fn file_dialog(request: &PickRequest, parent: &Parent) -> rfd::FileDialog {
    let d = match parent {
        Some(w) if !cfg!(target_os = "macos") => rfd::FileDialog::new().set_parent(&**w),
        _ => rfd::FileDialog::new(),
    };
    let pick = match request {
        PickRequest::Open(pick) | PickRequest::Save(pick) => pick,
        PickRequest::OpenMany => {
            return fileio::place_filters().fold(d.set_title("Place"), |d, (name, exts)| d.add_filter(name, exts));
        }
        PickRequest::Folder => return d,
    };
    let d = pick.filters.iter().fold(d, |d, (name, exts)| d.add_filter(*name, exts));
    let d = match &pick.folder {
        Some(folder) => d.set_directory(folder),
        None => d,
    };
    if pick.name.is_empty() { d } else { d.set_file_name(&pick.name) }
}

/// Show `dialog` for `request` (on the calling thread, until it closes) → the paths picked.
fn show_dialog(dialog: rfd::FileDialog, request: &PickRequest) -> PickAnswer {
    let paths = match request {
        PickRequest::Open(_) => dialog.pick_file().into_iter().collect(),
        PickRequest::OpenMany => dialog.pick_files().unwrap_or_default(),
        PickRequest::Save(pick) => {
            // The folder a template suggests may not exist yet.
            if let Some(folder) = &pick.folder {
                let _ = std::fs::create_dir_all(folder);
            }
            dialog.save_file().into_iter().collect()
        }
        PickRequest::Folder => dialog.pick_folder().into_iter().collect(),
    };
    paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect()
}

/// Show the dialog for `request` over `parent` now → the paths picked.
fn pick_now(request: PickRequest, parent: &Parent) -> PickAnswer {
    show_dialog(file_dialog(&request, parent), &request)
}

/// Linux and macOS: show the dialog for `request` over `parent` on a thread of its
/// own, answering on the receiver. Shown on the UI thread nothing would answer the
/// compositor meanwhile on Linux, which then offers to kill the window as not
/// responding; on macOS the panel ran modally inside the window's event handler.
fn start_pick(request: PickRequest, parent: &Parent) -> Option<std::sync::mpsc::Receiver<PickAnswer>> {
    let dialog = file_dialog(&request, parent);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("file-dialog".into())
        .spawn(move || {
            // The app gone meanwhile has no use for the answer.
            let _ = tx.send(show_dialog(dialog, &request));
        })
        .ok()?;
    Some(rx)
}

/// Write a file the safe way: a failed write keeps the old file.
fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    fileio::write_atomic(std::path::Path::new(path), bytes).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &str) -> std::process::Command {
    let mut c = std::process::Command::new("open");
    c.args(["-R", path]);
    c
}

#[cfg(windows)]
fn reveal_command(path: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt as _;
    // Explorer reads `/select,"path"` itself and needs backslashes.
    let mut c = std::process::Command::new("explorer");
    c.raw_arg(format!("/select,\"{}\"", path.replace('/', "\\")));
    c
}

#[cfg(not(any(target_os = "macos", windows)))]
fn reveal_command(path: &str) -> std::process::Command {
    let folder = std::path::Path::new(path).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| std::path::Path::new("."));
    let mut c = std::process::Command::new("xdg-open");
    c.arg(folder);
    c
}

/// File ▸ Show in Folder: select `path` in Finder / Explorer, or open its folder.
fn reveal(path: &str) -> Result<(), String> {
    reveal_command(path).spawn().map(|_| ()).map_err(|e| format!("can't show {path}: {e}"))
}

/// The platform callbacks the UI draws through: file dialogs, reading and writing
/// files, Help links.
///
/// Trowel installs no `system_clipboard` and no `print` yet, so Copy and Paste work
/// through egui's own clipboard (text and SVG) and File ▸ Print falls back to
/// saving a PDF. Both are J003 follow-ups, not part of the window arrangement.
fn services(parent: Parent) -> Services {
    let (p1, p2, p3, p4) = (parent.clone(), parent.clone(), parent.clone(), parent.clone());
    let start_pick: vectorcraft_ui_egui::picks::StartPick = cfg!(target_os = "macos")
        .then(|| Box::new(move |request: PickRequest| start_pick(request, &p4)) as vectorcraft_ui_egui::picks::StartPick)
        .unwrap_or_else(|| Box::new(|_request: PickRequest| -> Option<std::sync::mpsc::Receiver<PickAnswer>> { None }));
    Services {
        pick_open: Some(Box::new(move |pick: &FilePick| pick_now(PickRequest::Open(pick.clone()), &p1).into_iter().next())),
        pick_open_multi: Some(Box::new(move || pick_now(PickRequest::OpenMany, &p2))),
        pick_save: Some(Box::new(move |pick: &FilePick| pick_now(PickRequest::Save(pick.clone()), &p3).into_iter().next())),
        pick_folder: Some(Box::new(move || pick_now(PickRequest::Folder, &parent).into_iter().next())),
        read: Some(Box::new(|p: &str| std::fs::read(p).map_err(|e| e.to_string()))),
        write: Some(Box::new(write_file)),
        write_shared: Some(std::sync::Arc::new(write_file)),
        start_pick: Some(start_pick),
        open_url: Some(Box::new(|url: &str| {
            let _ = webbrowser::open(url);
        })),
        reveal: Some(Box::new(reveal)),
        ..Default::default()
    }
}

/// Windows and Linux draw their own title bar (the app bar is the title bar), as
/// the VectorCraft app does; macOS keeps its traffic lights.
const CUSTOM_TITLEBAR: bool = !cfg!(target_os = "macos");

fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Trowel")
            .with_inner_size(window::DEFAULT_SIZE)
            .with_min_inner_size(window::MIN_SIZE)
            .with_drag_and_drop(true)
            .with_decorations(!CUSTOM_TITLEBAR)
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        #[cfg(feature = "windows7")]
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    }
}

struct App {
    app: VectorcraftApp,
    /// Frames the interface has run. egui rebuilds the font set during the frame it is set in, so the
    /// app's own faces are only in place from the second frame on.
    frames: u64,
    /// Installed fonts covering what the interface fonts lack (Hangul).
    fonts: shell::fonts::Fallback,
}

impl eframe::App for App {
    // Each frame runs unconfined: it is the person at the keyboard's. The app
    // confines what the control channel asks for to its automation roots.
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        file_access::unconfined(|| self.logic_frame(ctx, frame));
    }
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        file_access::unconfined(|| self.app.raw_input_hook(raw));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // egui rebuilds the font set during the frame it is set in, so the app's own
        // faces (ui-semibold, ui, mono) are in place from the second frame on. The
        // first frame draws nothing and asks for another; VectorCraft's own frame gate
        // does the same with its fonts_ready.
        if self.frames < 2 {
            ui.ctx().request_repaint();
            return;
        }
        let mut fonts = std::mem::take(&mut self.fonts);
        file_access::unconfined(|| shell::trowel_frame(&mut self.app, ui, &mut fonts));
        self.fonts = fonts;
    }
    fn on_exit(&mut self) {
        file_access::unconfined(|| save_prefs(&self.app));
    }
}

impl App {
    fn logic_frame(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        self.app.logic(ctx);
        // The caption reads the drawing, a `*` when it has unsaved changes, then the
        // product — design 2.1 section 3.
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(shell::drawing_book::titled(&self.app, "Trowel")));
        if self.app.ui.status == "quit" {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn main() -> std::process::ExitCode {
    // First, so every start-up warning is recorded.
    logging::install(log_dir());
    vectorcraft_ui_egui::i18n::detect_system_lang_in_background();

    let control_port = std::env::var("TROWEL_CONTROL_PORT").ok().and_then(|v| v.parse().ok());
    let files: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect();
    let saved = read_prefs();

    let outcome = vectorcraft_engine::guard::catch_panic(|| {
        eframe::run_native(
            "Trowel",
            native_options(),
            Box::new(move |cc| {
                let mut app = VectorcraftApp::new(Session::new(), services(cc.winit_window().cloned()));
                load_prefs(&mut app, saved.clone());
                app.custom_titlebar = CUSTOM_TITLEBAR;
                if let Some(port) = control_port {
                    let rx = control_server::start(port, cc.egui_ctx.clone());
                    app = app.with_control(rx);
                }
                let mut app = app;
                for f in &files {
                    if let Err(e) = vectorcraft_ui_egui::io::open_reporting(&mut app, f) {
                        eprintln!("trowel: {f}: {e}");
                    }
                }
                Ok(Box::new(App { app, frames: 0, fonts: shell::fonts::Fallback::new() }) as Box<dyn eframe::App>)
            }),
        )
    });

    match outcome {
        Ok(Ok(())) => std::process::ExitCode::SUCCESS,
        // eframe hands back an error when the window could not be created at all
        // (no display, the adapter list empty). Report it instead of a bare code.
        Ok(Err(e)) => {
            log::error!("trowel: the window could not be opened: {e}");
            eprintln!("trowel: the window could not be opened: {e}");
            std::process::ExitCode::FAILURE
        }
        // `guard::catch_panic` reports what it recovered from; it never re-raises.
        Err(e) => {
            log::error!("trowel: {e}");
            eprintln!("trowel: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The window opens at the size the design 2.1 mock-up shows (section 3:
    /// 1600 × 988), and never below the minimum the layout needs.
    #[test]
    fn window_sizes_match_the_mock_up() {
        assert_eq!(window::DEFAULT_SIZE.x, 1600.0);
        assert_eq!(window::DEFAULT_SIZE.y, 988.0);
        const { assert!(window::MIN_SIZE.x < window::DEFAULT_SIZE.x) };
        const { assert!(window::MIN_SIZE.y < window::DEFAULT_SIZE.y) };
    }

    /// The title bar is the app bar on Windows and Linux, and the system's on macOS.
    #[test]
    fn custom_titlebar_follows_the_platform() {
        assert_eq!(CUSTOM_TITLEBAR, !cfg!(target_os = "macos"));
    }
}
