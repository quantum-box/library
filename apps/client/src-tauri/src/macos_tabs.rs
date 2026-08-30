//! macOS window tabs backed by Tauri child WebViews.
//!
//! One native window ("main") hosts every tab. Each tab is a child WebView of
//! that window, so switching tabs never hides or recreates the window itself.
//! The pattern, including the two-frame readiness handshake that keeps a fresh
//! WebView's white backing layer off screen, follows
//! `@tachyon-sdk/native-ui` `docs/tauri-macos-tabs.md`.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use tauri::{
    utils::config::WebviewUrl, AppHandle, Emitter, Manager,
    PhysicalPosition, PhysicalSize, Runtime, Webview, WebviewBuilder,
};

pub const NEW_TAB_MENU_ID: &str = "new_tab";
const TAB_WEBVIEW_PREFIX: &str = "library-tab-";
const TABS_CHANGED_EVENT: &str = "library-tabs-changed";
const MAIN_WINDOW_LABEL: &str = "main";
const DEFAULT_TAB_TITLE: &str = "Library";

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTab {
    label: String,
    title: String,
    selected: bool,
}

pub struct WindowTabsState {
    selected_label: Mutex<String>,
    titles: Mutex<HashMap<String, String>>,
    ready_labels: Mutex<HashSet<String>>,
    activate_when_ready: Mutex<HashSet<String>>,
}

impl Default for WindowTabsState {
    fn default() -> Self {
        Self {
            selected_label: Mutex::new(MAIN_WINDOW_LABEL.to_string()),
            titles: Mutex::new(HashMap::from([(
                MAIN_WINDOW_LABEL.to_string(),
                DEFAULT_TAB_TITLE.to_string(),
            )])),
            ready_labels: Mutex::new(HashSet::from([
                MAIN_WINDOW_LABEL.to_string()
            ])),
            activate_when_ready: Mutex::new(HashSet::new()),
        }
    }
}

impl WindowTabsState {
    pub fn selected_label(&self) -> String {
        self.selected_label.lock().unwrap().clone()
    }

    fn select(&self, label: &str) {
        *self.selected_label.lock().unwrap() = label.to_string();
    }

    fn title(&self, label: &str) -> String {
        self.titles
            .lock()
            .unwrap()
            .get(label)
            .cloned()
            .unwrap_or_else(|| DEFAULT_TAB_TITLE.to_string())
    }

    fn set_title(&self, label: &str, title: String) {
        self.titles.lock().unwrap().insert(label.to_string(), title);
    }

    fn is_ready(&self, label: &str) -> bool {
        self.ready_labels.lock().unwrap().contains(label)
    }

    fn mark_ready(&self, label: &str) {
        self.ready_labels.lock().unwrap().insert(label.to_string());
    }

    fn set_activate_when_ready(&self, label: &str, activate: bool) {
        if activate {
            self.activate_when_ready
                .lock()
                .unwrap()
                .insert(label.to_string());
        }
    }

    fn take_activate_when_ready(&self, label: &str) -> bool {
        self.activate_when_ready.lock().unwrap().remove(label)
    }

    fn remove(&self, label: &str) {
        self.titles.lock().unwrap().remove(label);
        self.ready_labels.lock().unwrap().remove(label);
        self.activate_when_ready.lock().unwrap().remove(label);
    }
}

fn next_tab_label() -> String {
    let id = NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed);
    format!("{TAB_WEBVIEW_PREFIX}{id}")
}

/// Only in-app paths may seed a child WebView, so a tab can never be pointed at
/// an external origin.
fn sanitize_tab_path(path: &str) -> Option<String> {
    if !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    if path.contains("://") || path.contains('\\') {
        return None;
    }
    Some(path.to_string())
}

/// Creates a child WebView off screen so its first paint never flashes white
/// over the current tab. `finish_loading_tab` moves it into place.
pub fn open_new_tab<R: Runtime>(
    app: &AppHandle<R>,
    path: Option<&str>,
    activate: bool,
) -> tauri::Result<()> {
    let window = app
        .get_window(MAIN_WINDOW_LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;
    let tabs_state = app.state::<WindowTabsState>();
    let mut window_config = app
        .config()
        .app
        .windows
        .first()
        .cloned()
        .ok_or(tauri::Error::WindowNotFound)?;
    let label = next_tab_label();
    window_config.label.clone_from(&label);
    if let Some(path) = path.and_then(sanitize_tab_path) {
        window_config.url = WebviewUrl::App(path.into());
    }
    tabs_state.set_title(&label, DEFAULT_TAB_TITLE.to_string());
    tabs_state.set_activate_when_ready(&label, activate);
    let size = window.inner_size()?;
    let offscreen_x = -i32::try_from(size.width).unwrap_or(i32::MAX);
    let builder =
        WebviewBuilder::from_config(&window_config).focused(false);
    if let Err(error) = window.add_child(
        builder,
        PhysicalPosition::new(offscreen_x, 0),
        PhysicalSize::new(size.width, size.height),
    ) {
        tabs_state.remove(&label);
        return Err(error);
    }

    Ok(())
}

fn finish_loading_tab<R: Runtime>(
    app: &AppHandle<R>,
    tab: &Webview<R>,
    label: &str,
) -> tauri::Result<()> {
    let tabs_state = app.state::<WindowTabsState>();
    let activate = tabs_state.take_activate_when_ready(label);

    if activate {
        let current_label = tabs_state.selected_label();
        let current = app.get_webview(&current_label);
        tab.set_position(PhysicalPosition::new(0, 0))?;
        tab.set_auto_resize(true)?;
        tabs_state.mark_ready(label);
        tab.show()?;
        tab.set_focus()?;
        if let Some(current) = current {
            if current.label() != label {
                current.hide()?;
            }
        }
        tabs_state.select(label);
        tab.window().set_title(&tabs_state.title(label))?;
        emit_tabs_changed(app)
    } else {
        tab.hide()?;
        tab.set_position(PhysicalPosition::new(0, 0))?;
        tab.set_auto_resize(true)?;
        tabs_state.mark_ready(label);
        emit_tabs_changed(app)
    }
}

/// Shows the target before hiding the previous WebView so no frame exposes the
/// empty window behind them.
fn show_tab_in_place<R: Runtime>(
    current: &Webview<R>,
    target: &Webview<R>,
) -> tauri::Result<()> {
    if current.label() == target.label() {
        return Ok(());
    }

    target.show()?;
    target.set_focus()?;
    current.hide()?;

    Ok(())
}

fn activate_tab<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
) -> tauri::Result<()> {
    let tabs_state = app.state::<WindowTabsState>();
    if !tabs_state.is_ready(label) {
        return Ok(());
    }

    let current = app
        .get_webview(&tabs_state.selected_label())
        .ok_or(tauri::Error::WindowNotFound)?;
    let target =
        app.get_webview(label).ok_or(tauri::Error::WindowNotFound)?;
    show_tab_in_place(&current, &target)?;
    tabs_state.select(label);
    target.window().set_title(&tabs_state.title(label))?;
    emit_tabs_changed(app)
}

fn tab_sort_key(label: &str) -> u64 {
    if label == MAIN_WINDOW_LABEL {
        0
    } else {
        label
            .strip_prefix(TAB_WEBVIEW_PREFIX)
            .and_then(|id| id.parse::<u64>().ok())
            .unwrap_or(u64::MAX)
    }
}

fn is_tab_label(label: &str) -> bool {
    label == MAIN_WINDOW_LABEL || label.starts_with(TAB_WEBVIEW_PREFIX)
}

pub fn emit_tabs_changed<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri::Result<()> {
    app.emit(TABS_CHANGED_EVENT, ())
}

fn sorted_tab_webviews<R: Runtime>(app: &AppHandle<R>) -> Vec<Webview<R>> {
    let mut webviews = app
        .webviews()
        .into_values()
        .filter(|webview| is_tab_label(webview.label()))
        .collect::<Vec<_>>();
    webviews.sort_by_key(|webview| tab_sort_key(webview.label()));
    webviews
}

fn preferred_successor_label<'a>(
    tabs: impl Iterator<Item = (&'a str, bool)>,
    closing_label: &str,
) -> Option<String> {
    let candidates = tabs
        .filter(|(label, _)| *label != closing_label)
        .collect::<Vec<_>>();
    candidates
        .iter()
        .find(|(_, ready)| *ready)
        .or_else(|| candidates.first())
        .map(|(label, _)| (*label).to_string())
}

pub fn close_tab_webview<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
) -> tauri::Result<()> {
    let tabs_state = app.state::<WindowTabsState>();
    let tabs = sorted_tab_webviews(app);
    if tabs.len() == 1 {
        tabs[0].window().destroy()?;
        return Ok(());
    }

    let target =
        app.get_webview(label).ok_or(tauri::Error::WindowNotFound)?;

    if label == tabs_state.selected_label() {
        let next_label = preferred_successor_label(
            tabs.iter()
                .map(|tab| (tab.label(), tabs_state.is_ready(tab.label()))),
            label,
        )
        .ok_or(tauri::Error::WindowNotFound)?;
        let next = app
            .get_webview(&next_label)
            .ok_or(tauri::Error::WindowNotFound)?;
        if tabs_state.is_ready(next.label()) {
            show_tab_in_place(&target, &next)?;
            tabs_state.select(next.label());
            next.window().set_title(&tabs_state.title(next.label()))?;
        } else {
            tabs_state.set_activate_when_ready(next.label(), true);
            tabs_state.select(next.label());
            next.window().set_title(&tabs_state.title(next.label()))?;
        }
    }

    target.close()?;
    tabs_state.remove(label);
    emit_tabs_changed(app)
}

#[tauri::command]
pub fn list_window_tabs(app: AppHandle) -> Vec<WindowTab> {
    let tabs_state = app.state::<WindowTabsState>();
    let selected_label = tabs_state.selected_label();
    sorted_tab_webviews(&app)
        .into_iter()
        .map(|tab| WindowTab {
            label: tab.label().to_string(),
            title: tabs_state.title(tab.label()),
            selected: tab.label() == selected_label,
        })
        .collect()
}

#[tauri::command]
pub fn create_window_tab(
    app: AppHandle,
    path: Option<String>,
    activate: bool,
) -> tauri::Result<()> {
    open_new_tab(&app, path.as_deref(), activate)?;
    emit_tabs_changed(&app)
}

#[tauri::command]
pub fn mark_window_tab_content_ready(
    app: AppHandle,
    webview: Webview,
) -> tauri::Result<()> {
    let tabs_state = app.state::<WindowTabsState>();
    if tabs_state.is_ready(webview.label()) {
        return Ok(());
    }

    finish_loading_tab(&app, &webview, webview.label())
}

#[tauri::command]
pub fn activate_window_tab(
    app: AppHandle,
    label: String,
) -> tauri::Result<()> {
    activate_tab(&app, &label)
}

#[tauri::command]
pub fn close_window_tab(
    app: AppHandle,
    label: String,
) -> tauri::Result<()> {
    close_tab_webview(&app, &label)
}

#[tauri::command]
pub fn update_window_tab_title(
    app: AppHandle,
    webview: Webview,
    title: String,
) -> tauri::Result<()> {
    let tabs_state = app.state::<WindowTabsState>();
    tabs_state.set_title(webview.label(), title.clone());
    if webview.label() == tabs_state.selected_label() {
        webview.window().set_title(&title)?;
    }
    emit_tabs_changed(&app)
}

#[cfg(test)]
mod tests {
    use super::{
        next_tab_label, preferred_successor_label, sanitize_tab_path,
        tab_sort_key, WindowTabsState, TAB_WEBVIEW_PREFIX,
    };

    #[test]
    fn tab_webview_labels_are_unique_and_prefixed() {
        let first = next_tab_label();
        let second = next_tab_label();

        assert!(first.starts_with(TAB_WEBVIEW_PREFIX));
        assert!(second.starts_with(TAB_WEBVIEW_PREFIX));
        assert_ne!(first, second);
    }

    #[test]
    fn tab_sort_keeps_main_before_dynamic_tabs() {
        assert_eq!(tab_sort_key("main"), 0);
        assert_eq!(tab_sort_key("library-tab-2"), 2);
        assert_eq!(tab_sort_key("library-tab-10"), 10);
    }

    #[test]
    fn tab_is_activatable_only_after_initial_load() {
        let state = WindowTabsState::default();

        assert!(state.is_ready("main"));
        assert!(!state.is_ready("library-tab-1"));

        state.mark_ready("library-tab-1");
        assert!(state.is_ready("library-tab-1"));

        state.remove("library-tab-1");
        assert!(!state.is_ready("library-tab-1"));
    }

    #[test]
    fn foreground_activation_is_consumed_after_first_paint() {
        let state = WindowTabsState::default();

        state.set_activate_when_ready("library-tab-1", true);

        assert!(state.take_activate_when_ready("library-tab-1"));
        assert!(!state.take_activate_when_ready("library-tab-1"));
    }

    #[test]
    fn close_prefers_ready_successor_but_falls_back_to_loading_tab() {
        let ready = preferred_successor_label(
            [
                ("main", true),
                ("library-tab-1", false),
                ("library-tab-2", true),
            ]
            .into_iter(),
            "main",
        );
        assert_eq!(ready.as_deref(), Some("library-tab-2"));

        let loading = preferred_successor_label(
            [("main", true), ("library-tab-1", false)].into_iter(),
            "main",
        );
        assert_eq!(loading.as_deref(), Some("library-tab-1"));
    }

    #[test]
    fn only_in_app_paths_can_seed_a_tab() {
        assert_eq!(
            sanitize_tab_path("/acme/docs/data").as_deref(),
            Some("/acme/docs/data")
        );
        assert_eq!(sanitize_tab_path("https://example.com"), None);
        assert_eq!(sanitize_tab_path("//example.com"), None);
        assert_eq!(sanitize_tab_path("acme/docs"), None);
    }
}
