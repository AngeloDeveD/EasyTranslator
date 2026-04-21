// В release-сборке на Windows отключаем дополнительное консольное окно.
// Это стандартный паттерн для desktop-приложений на Tauri.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


fn main() {
    // Весь runtime приложения и регистрация команд находятся в lib.rs.
    smart_translator_lib::run()
}
