// Главный backend-модуль Tauri.
// Здесь инициализируется инфраструктура приложения и регистрируются команды,
// которые вызывает React-часть через `invoke`.
mod db;
mod installer;
mod downloader;

use std::fs;
use rfd::FileDialog;
use tauri::Manager;
// TODO: при необходимости вернуть window-vibrancy в setup для кастомных эффектов окна.

#[tauri::command]
fn open_file() -> Result<String, String> {
    // Простой helper-командлет для тестового текстового редактора в UI.
    let file_path = FileDialog::new()
        .add_filter("Текстовые файлы", &["txt", "md", "csv", "rs"])
        .pick_file();

    if let Some(path) = file_path{
        match fs::read_to_string(path){
            Ok(content) => Ok(content),
            Err(e) => Err(format!("Ошибка чтения: {}", e)),
        }
    }
    else {
        Err(String::from("Выбор файла отменён"))
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    // Базовая тестовая команда из шаблона Tauri.
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn save_file(content: String) -> Result<String, String>{
    // Простой helper-командлет для тестового текстового редактора в UI.
    let file_path = FileDialog::new()
        .add_filter("Текстовый файл", &["txt"])
        .save_file();

    if let Some(path) = file_path{
        match fs::write(&path, content) {
            Ok(_) => Ok(format!("Успешно сохранено в {:?}", path)),
            Err(e) => Err(format!("Ошибка сохранения: {}", e)),
        }
    }
    else{
        Err(String::from("Сохранение отменено"))
    }
    
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // setup выполняется один раз перед стартом приложения.
        .setup(|app| {
            // База данных хранится в системной app-data папке приложения.
            let app_dir = app.path().app_data_dir()
                .expect("Не удалось получить папку с данными");

            println!("Папка с данными приложения: {:?}", app_dir);

            // Инициализация SQLite и регистрация соединения в глобальном состоянии Tauri.
            let conn = db::init(app_dir)
                .expect("КРИТ. ОШИБКА: НЕ УДАЛОСЬ ИНИЦИАЛИЗИРОВАТЬ БД");

            app.manage(db::DbState(std::sync::Mutex::new(conn)));

            // Важно вернуть `Ok(())` без `;` внутри closure.
            Ok(())

        })
        .plugin(tauri_plugin_opener::init())
        // Единая точка экспорта backend-команд для frontend.
        .invoke_handler(tauri::generate_handler![
            greet, open_file, save_file, 
            db::get_games, 
            db::sync_catalog, 
            db::set_game_path, 
            db::auto_detect_game_path,
            db::reset_game_path, 
            db::install_localization, 
            db::get_localizations,
            db::delete_localization,
            db::add_local_game,
            db::add_local_localization,
            db::pick_localization_file,
            db::disable_localization,
            db::delete_localization,
            
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
