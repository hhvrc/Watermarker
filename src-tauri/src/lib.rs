mod commands;
mod compositor;
mod decode;
mod export;
mod metadata;
mod placement;
mod preview;
mod settings;
mod state;
mod watermark;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init());

    // The updater plugin is desktop-only.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::import_images,
            commands::import_image_bytes,
            commands::remove_image,
            commands::get_image_preview,
            commands::set_watermark,
            commands::set_watermark_persisted,
            commands::set_watermark_bytes,
            commands::get_watermark_preview,
            commands::render_exact_preview,
            commands::get_settings,
            commands::set_settings,
            export::process_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
