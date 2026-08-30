//! The macOS menu bar.
//!
//! macOS localizes none of this for us. Every entry `Menu::default` would
//! build — `Services`, `Quit …`, `Edit`, `Window` — is an English literal in
//! `muda`, and the bundle ships no `.lproj` translations, so the standard
//! items are no more translated than the two the app owns
//! (`File ▸ New Tab` and `Library ▸ Check for Updates…`).
//!
//! The bar is therefore assembled here in English, the app's source language,
//! and the frontend pushes translated labels down through `set_menu_labels`
//! as soon as it has a catalog — and again whenever the reader switches
//! language. Only the app submenu's own title stays put: it is the product
//! name, which is not translated.

use tauri::{
    menu::{
        AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu,
        HELP_SUBMENU_ID, WINDOW_SUBMENU_ID,
    },
    AppHandle, Emitter, Manager, Runtime,
};

use crate::macos_tabs::{WindowTabsState, NEW_TAB_MENU_ID};

pub const CHECK_FOR_UPDATES_MENU_ID: &str = "check_for_updates";

/// Mirrors `CHECK_FOR_UPDATES_EVENT` in `src/lib/appUpdate.ts`.
const CHECK_FOR_UPDATES_EVENT: &str = "library-check-for-updates";

/// Every label the bar shows, as `src/lib/desktop/menuLabels.ts` sends them.
///
/// Names have to line up with the `MenuLabels` interface there. Both a missing
/// and an unexpected field fail the command outright, so a rename on one side
/// surfaces as an error instead of an entry silently stuck in English.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MenuLabels {
    about: String,
    check_for_updates: String,
    services: String,
    hide: String,
    hide_others: String,
    quit: String,
    file: String,
    new_tab: String,
    close_window: String,
    edit: String,
    undo: String,
    redo: String,
    cut: String,
    copy: String,
    paste: String,
    select_all: String,
    view: String,
    toggle_full_screen: String,
    window: String,
    minimize: String,
    zoom: String,
    help: String,
}

/// Handles to every item the frontend retitles.
///
/// They are kept rather than looked up again because `Menu::get` only searches
/// one level down and predefined items carry generated ids, so nothing in the
/// submenus can be found by id afterwards.
struct MenuHandles<R: Runtime> {
    about: PredefinedMenuItem<R>,
    check_for_updates: MenuItem<R>,
    services: PredefinedMenuItem<R>,
    hide: PredefinedMenuItem<R>,
    hide_others: PredefinedMenuItem<R>,
    quit: PredefinedMenuItem<R>,
    file: Submenu<R>,
    new_tab: MenuItem<R>,
    /// `Close Window` sits in both the File and the Window menu.
    close_window: [PredefinedMenuItem<R>; 2],
    edit: Submenu<R>,
    undo: PredefinedMenuItem<R>,
    redo: PredefinedMenuItem<R>,
    cut: PredefinedMenuItem<R>,
    copy: PredefinedMenuItem<R>,
    paste: PredefinedMenuItem<R>,
    select_all: PredefinedMenuItem<R>,
    view: Submenu<R>,
    toggle_full_screen: PredefinedMenuItem<R>,
    window: Submenu<R>,
    minimize: PredefinedMenuItem<R>,
    zoom: PredefinedMenuItem<R>,
    help: Submenu<R>,
}

impl<R: Runtime> MenuHandles<R> {
    fn apply(&self, labels: &MenuLabels) -> tauri::Result<()> {
        self.about.set_text(&labels.about)?;
        self.check_for_updates.set_text(&labels.check_for_updates)?;
        self.services.set_text(&labels.services)?;
        self.hide.set_text(&labels.hide)?;
        self.hide_others.set_text(&labels.hide_others)?;
        self.quit.set_text(&labels.quit)?;
        self.file.set_text(&labels.file)?;
        self.new_tab.set_text(&labels.new_tab)?;
        for item in &self.close_window {
            item.set_text(&labels.close_window)?;
        }
        self.edit.set_text(&labels.edit)?;
        self.undo.set_text(&labels.undo)?;
        self.redo.set_text(&labels.redo)?;
        self.cut.set_text(&labels.cut)?;
        self.copy.set_text(&labels.copy)?;
        self.paste.set_text(&labels.paste)?;
        self.select_all.set_text(&labels.select_all)?;
        self.view.set_text(&labels.view)?;
        self.toggle_full_screen
            .set_text(&labels.toggle_full_screen)?;
        self.window.set_text(&labels.window)?;
        self.minimize.set_text(&labels.minimize)?;
        self.zoom.set_text(&labels.zoom)?;
        self.help.set_text(&labels.help)?;
        Ok(())
    }
}

/// The bundle's display name, so the frontend can fill `{name}` into the
/// `About …` / `Hide …` / `Quit …` labels without keeping a second copy of it
/// beside `tauri.conf.json`.
#[tauri::command]
pub fn app_product_name<R: Runtime>(app: AppHandle<R>) -> String {
    app.package_info().name.clone()
}

/// Retitles the whole bar in the language the frontend is rendering in.
#[tauri::command]
pub fn set_menu_labels<R: Runtime>(
    app: AppHandle<R>,
    labels: MenuLabels,
) -> tauri::Result<()> {
    app.state::<MenuHandles<R>>().apply(&labels)
}

/// Lays out the bar `Menu::default` would build, plus `New Tab` and
/// `Check for Updates…`, with every label held onto for `set_menu_labels`.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let package = app.package_info();
    let config = app.config();
    let about_metadata = AboutMetadata {
        name: Some(package.name.clone()),
        version: Some(package.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };

    let about = PredefinedMenuItem::about(app, None, Some(about_metadata))?;
    let check_for_updates = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_MENU_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let services = PredefinedMenuItem::services(app, None)?;
    let hide = PredefinedMenuItem::hide(app, None)?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let quit = PredefinedMenuItem::quit(app, None)?;
    let new_tab = MenuItem::with_id(
        app,
        NEW_TAB_MENU_ID,
        "New Tab",
        true,
        Some("CmdOrCtrl+T"),
    )?;
    let close_window = [
        PredefinedMenuItem::close_window(app, None)?,
        PredefinedMenuItem::close_window(app, None)?,
    ];
    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;
    let toggle_full_screen = PredefinedMenuItem::fullscreen(app, None)?;
    let minimize = PredefinedMenuItem::minimize(app, None)?;
    let zoom = PredefinedMenuItem::maximize(app, None)?;

    // The app submenu is titled with the product name, which stays in every
    // language. `Check for Updates…` goes right under About, where macOS apps
    // conventionally put it.
    let app_menu = Submenu::with_items(
        app,
        package.name.clone(),
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &check_for_updates,
            &PredefinedMenuItem::separator(app)?,
            &services,
            &PredefinedMenuItem::separator(app)?,
            &hide,
            &hide_others,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &new_tab,
            &PredefinedMenuItem::separator(app)?,
            &close_window[0],
        ],
    )?;

    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &PredefinedMenuItem::separator(app)?,
            &cut,
            &copy,
            &paste,
            &select_all,
        ],
    )?;

    let view =
        Submenu::with_items(app, "View", true, &[&toggle_full_screen])?;

    // Tauri hands the submenus carrying these two ids to AppKit as the
    // windows and help menus, which is what fills the Window menu with the
    // open windows and puts the search field in Help.
    let window = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &minimize,
            &zoom,
            &PredefinedMenuItem::separator(app)?,
            &close_window[1],
        ],
    )?;

    let help = Submenu::with_id_and_items(
        app,
        HELP_SUBMENU_ID,
        "Help",
        true,
        &[],
    )?;

    let menu = Menu::with_items(
        app,
        &[&app_menu, &file, &edit, &view, &window, &help],
    )?;

    app.manage(MenuHandles {
        about,
        check_for_updates,
        services,
        hide,
        hide_others,
        quit,
        file,
        new_tab,
        close_window,
        edit,
        undo,
        redo,
        cut,
        copy,
        paste,
        select_all,
        view,
        toggle_full_screen,
        window,
        minimize,
        zoom,
        help,
    });

    Ok(menu)
}

/// The menu bar is app-wide but every tab renders its own updater dialog, so
/// the request only goes to the tab in front.
pub fn request_update_check<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri::Result<()> {
    let label = app.state::<WindowTabsState>().selected_label();
    app.emit_to(label.as_str(), CHECK_FOR_UPDATES_EVENT, ())
}

#[cfg(test)]
mod tests {
    use super::MenuLabels;

    /// The payload `menuLabels()` builds, spelled the way it arrives over IPC.
    const LABELS: &str = r#"{
        "about": "Library について",
        "checkForUpdates": "アップデートを確認…",
        "services": "サービス",
        "hide": "Library を隠す",
        "hideOthers": "ほかを隠す",
        "quit": "Library を終了",
        "file": "ファイル",
        "newTab": "新規タブ",
        "closeWindow": "ウインドウを閉じる",
        "edit": "編集",
        "undo": "取り消す",
        "redo": "やり直す",
        "cut": "カット",
        "copy": "コピー",
        "paste": "ペースト",
        "selectAll": "すべてを選択",
        "view": "表示",
        "toggleFullScreen": "フルスクリーンを切り替える",
        "window": "ウインドウ",
        "minimize": "しまう",
        "zoom": "ズーム",
        "help": "ヘルプ"
    }"#;

    #[test]
    fn labels_arrive_under_the_names_the_frontend_sends() {
        let labels: MenuLabels = serde_json::from_str(LABELS).unwrap();

        assert_eq!(labels.check_for_updates, "アップデートを確認…");
        assert_eq!(labels.new_tab, "新規タブ");
        assert_eq!(labels.hide_others, "ほかを隠す");
        assert_eq!(labels.toggle_full_screen, "フルスクリーンを切り替える");
    }

    #[test]
    fn a_label_the_frontend_stops_sending_is_an_error_not_an_english_entry()
    {
        let without_zoom = LABELS.replace(r#""zoom": "ズーム","#, "");

        let error = serde_json::from_str::<MenuLabels>(&without_zoom)
            .err()
            .expect("a missing label must fail the command");
        assert!(error.to_string().contains("zoom"), "{error}");
    }
}
