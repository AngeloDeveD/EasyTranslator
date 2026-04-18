use rusqlite::{Connection, Result};
use std::path::PathBuf;

//Инициализация бд
pub fn init(app_data_dir: PathBuf) -> Result<Connection>{
    //Проверка на существование папки данных приложения
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir)
            .expect("Не удалось создать папку данных приложения");
    }

    //Формируем путь к файлу бд
    let db_path = app_data_dir.join("launcher.db");

    //Открываем или создаём файл бд
    let conn = Connection::open(db_path)?;

    //Настройки производительности и безопасности
    //WAL -> позвоялет читать и одновременно записывать данные в бд
    //Foreign Keys -> Включение поддержки внешних ключей

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;"
    )?;

    //Создание таблиц
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            install_path TEXT
        );

        CREATE TABLE IF NOT EXISTS localizations (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            version TEXT NOT NULL,
            primary_url TEXT NOT NULL,
            backup_url TEXT,
            archive_hash TEXT NOT NULL,
            file_size_mb INTEGER,
            install_instructions TEXT NOT NULL,
            dll_whitelist TEXT,
            FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
        
        );

        CREATE TABLE IF NOT EXISTS install_states (
            localization_id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'available',
            installed_version TEXT,
            backup_path TEXT,
            error_message TEXT,
            FOREIGN KEY (localization_id) REFERENCES localizations(id) ON DELETE CASCADE
        );
        "
    )?;

    Ok(conn)
}

/*Если удалить игру из таблицы, то автоматически удалятся
все её локализации из localizations и все статусы установок из install_states

launcher.db хранится в AppData
*/