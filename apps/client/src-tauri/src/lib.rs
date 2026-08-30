use photon_engine::{projection::apply_operation, Operation, Record};

#[cfg(target_os = "macos")]
mod macos_menu;

#[cfg(target_os = "macos")]
mod macos_tabs;

#[cfg(target_os = "macos")]
use tauri::{Manager, WindowEvent};

#[tauri::command]
fn photon_engine_apply_operation(
    current: Option<Record>,
    operation: Operation,
) -> Result<Record, String> {
    apply_operation(current, &operation).map_err(|error| error.to_string())
}

/// Lets the shared frontend enable platform-specific native controls without
/// sniffing the user agent.
#[tauri::command]
fn app_target_os() -> String {
    std::env::consts::OS.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().setup(|app| {
        #[cfg(desktop)]
        {
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            app.handle().plugin(tauri_plugin_process::init())?;
        }
        if cfg!(debug_assertions) {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;
        }
        Ok(())
    });

    #[cfg(target_os = "macos")]
    let builder = builder
        .manage(macos_tabs::WindowTabsState::default())
        .invoke_handler(tauri::generate_handler![
            photon_engine_apply_operation,
            app_target_os,
            macos_tabs::list_window_tabs,
            macos_tabs::create_window_tab,
            macos_tabs::mark_window_tab_content_ready,
            macos_tabs::activate_window_tab,
            macos_tabs::close_window_tab,
            macos_tabs::update_window_tab_title
        ])
        .menu(macos_menu::build)
        .on_menu_event(|app, event| {
            if event.id() == macos_tabs::NEW_TAB_MENU_ID {
                if let Err(error) =
                    macos_tabs::open_new_tab(app, None, true)
                {
                    log::error!("failed to open a new tab: {error}");
                } else if let Err(error) =
                    macos_tabs::emit_tabs_changed(app)
                {
                    log::error!("failed to emit tab change: {error}");
                }
            } else if event.id() == macos_menu::CHECK_FOR_UPDATES_MENU_ID {
                if let Err(error) = macos_menu::request_update_check(app) {
                    log::error!(
                        "failed to request an update check: {error}"
                    );
                }
            }
        })
        .on_window_event(|window, event| match event {
            // ⌘W and the window close button close the selected tab first; the
            // window itself only goes away with its last tab.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let app = window.app_handle();
                let label = app
                    .state::<macos_tabs::WindowTabsState>()
                    .selected_label();
                if let Err(error) =
                    macos_tabs::close_tab_webview(app, &label)
                {
                    log::error!("failed to close tab: {error}");
                }
            }
            WindowEvent::Destroyed => {
                if let Err(error) =
                    macos_tabs::emit_tabs_changed(window.app_handle())
                {
                    log::error!("failed to emit tab change: {error}");
                }
            }
            _ => {}
        });

    #[cfg(not(target_os = "macos"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        photon_engine_apply_operation,
        app_target_os
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
