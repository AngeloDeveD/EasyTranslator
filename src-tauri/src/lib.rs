// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs;
use rfd::FileDialog;
use tauri::Manager;
use window_vibrancy::{apply_blur, apply_mica, apply_vibrancy, NSVisualEffectMaterial};

#[tauri::command]
fn open_file() -> Result<String, String> {
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
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn save_file(content: String) -> Result<String, String>{
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
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Для macOS применяем эффект Vibrancy
            #[cfg(target_os = "macos")]
            {
                // Используем мягкую обработку ошибки через if let
                if let Err(e) = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None) {
                    eprintln!("Не удалось применить Vibrancy (macOS): {}", e);
                    // Приложение не крашится, просто идет дальше
                }
            }

            // Для Windows применяем эффект Mica/Blur
            #[cfg(target_os = "windows")]
            {
                if let Err(_) = apply_mica(&window, None) {
                    // 2. Если Mica выдала ошибку (это Windows 10), откатываемся к Blur
                    if let Err(e) = apply_blur(&window, Some((18, 18, 18, 125))) {
                        eprintln!("Не удалось применить ни Mica, ни Blur: {}", e);
                    }
                }
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_file, save_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
