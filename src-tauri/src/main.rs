// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::command;

#[tauri::command]
fn greet(name: &str) -> String{
    format!("Привет из Rust, {}!", name)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("Ошибка при запуске программы");

    smart_translator_lib::run()
}
