pub mod collect;
pub mod commands;
pub mod config;
pub mod error;
pub mod matching;
pub mod models;
pub mod providers;
pub mod scrape;
pub mod store;
pub mod sync;

use commands::AppState;
use store::Store;
use tauri::Manager;

pub fn run() {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter("chartographer_lib=debug")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = commands::data_dir(app.handle());
            std::fs::create_dir_all(&data_dir).ok();
            let store = Store::open(&config::db_path(&data_dir))?;
            app.manage(AppState { store, data_dir });

            // Sonde de diagnostic : lancée avec CF_PROBE=1, elle ouvre le
            // tableau de bord CurseForge, note ce que la page rend vraiment et
            // écrit son rapport à côté de la base. Sans elle, on coderait à
            // l'aveugle sur un site qu'on ne peut pas ouvrir autrement.
            if std::env::var("CF_PROBE").is_ok() {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    commands::probe_curseforge(handle).await;
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth_status,
            commands::connect,
            commands::logout,
            commands::open_token_page,
            commands::get_settings,
            commands::save_settings,
            commands::sync_now,
            commands::overview,
            commands::project_detail,
            commands::link_manual,
            commands::unlink,
            commands::set_solo,
            commands::pairing_state,
            commands::record_curseforge_points,
            commands::forget_curseforge_points,
            commands::curseforge_points,
            commands::open_curseforge_site,
            commands::open_curseforge_window,
            commands::arm_curseforge_capture,
            commands::collect_curseforge,
            commands::read_curseforge_page,
            commands::import_curseforge_capture,
            commands::analyze_curseforge_text,
            commands::import_curseforge_series,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de Tauri");
}
