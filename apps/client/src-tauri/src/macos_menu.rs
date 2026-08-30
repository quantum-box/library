//! The macOS menu bar.
//!
//! `Menu::default` already lays out the standard app / File / Edit / … menus;
//! this adds the two entries the app owns — `File ▸ New Tab` and
//! `Library Client ▸ Check for Updates…`.

use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, Manager, Runtime,
};

use crate::macos_tabs::{WindowTabsState, NEW_TAB_MENU_ID};

pub const CHECK_FOR_UPDATES_MENU_ID: &str = "check_for_updates";

/// Mirrors `CHECK_FOR_UPDATES_EVENT` in `src/lib/appUpdate.ts`.
const CHECK_FOR_UPDATES_EVENT: &str = "library-check-for-updates";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    add_new_tab(app, &menu)?;
    add_check_for_updates(app, &menu)?;
    Ok(menu)
}

/// Adds `File ▸ New Tab (⌘T)`.
fn add_new_tab<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Menu<R>,
) -> tauri::Result<()> {
    let new_tab = MenuItem::with_id(
        app,
        NEW_TAB_MENU_ID,
        "New Tab",
        true,
        Some("CmdOrCtrl+T"),
    )?;
    let separator = PredefinedMenuItem::separator(app)?;

    if let Some(file_menu) = submenu_named(menu, "File")? {
        file_menu.prepend_items(&[&new_tab, &separator])?;
    } else {
        let file_menu = Submenu::with_items(
            app,
            "File",
            true,
            &[&new_tab, &separator],
        )?;
        menu.prepend(&file_menu)?;
    }

    Ok(())
}

/// Adds `Library Client ▸ Check for Updates…` just below the About item, where
/// macOS apps conventionally put it.
fn add_check_for_updates<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Menu<R>,
) -> tauri::Result<()> {
    let Some(app_menu) = app_submenu(menu)? else {
        return Ok(());
    };

    let check = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_MENU_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;

    // Index 1 is right after About; a shorter menu means there is no About to
    // sit under, so append instead of failing.
    if app_menu.items()?.len() > 1 {
        app_menu.insert_items(&[&separator, &check], 1)?;
    } else {
        app_menu.append_items(&[&separator, &check])?;
    }

    Ok(())
}

/// The menu bar is app-wide but every tab renders its own updater dialog, so
/// the request only goes to the tab in front.
pub fn request_update_check<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri::Result<()> {
    let label = app.state::<WindowTabsState>().selected_label();
    app.emit_to(label.as_str(), CHECK_FOR_UPDATES_EVENT, ())
}

/// macOS always puts the app menu first, and it is titled with the product
/// name rather than anything we can match on reliably.
fn app_submenu<R: Runtime>(
    menu: &Menu<R>,
) -> tauri::Result<Option<Submenu<R>>> {
    Ok(menu.items()?.into_iter().find_map(|item| match item {
        MenuItemKind::Submenu(submenu) => Some(submenu),
        _ => None,
    }))
}

fn submenu_named<R: Runtime>(
    menu: &Menu<R>,
    text: &str,
) -> tauri::Result<Option<Submenu<R>>> {
    Ok(menu.items()?.into_iter().find_map(|item| match item {
        MenuItemKind::Submenu(submenu)
            if submenu.text().ok().as_deref() == Some(text) =>
        {
            Some(submenu)
        }
        _ => None,
    }))
}
