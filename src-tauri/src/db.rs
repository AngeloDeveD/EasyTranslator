use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use serde::Deserialize;
use tauri::State;
use rfd::FileDialog;
use tauri::AppHandle;

// --- СТРУКТУРЫ ДЛЯ ВЫВОДА (во фронтенд) ---

#[derive(serde::Serialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub install_path: Option<String>,
}

#[derive(serde::Serialize)]
pub struct Localization {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub file_size_mb: i64,
    pub status: String, 
}

// --- СТРУКТУРЫ ДЛЯ ПАРСИНГА (из JSON) ---

#[derive(Deserialize)]
pub struct CatalogGame {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub localizations: Vec<CatalogLocalization>,
}

#[derive(Deserialize)]
pub struct CatalogLocalization {
    pub id: String,
    pub name: String, // Название самого перевода
    pub version: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub primary_url: String,
    pub backup_url: Option<String>,
    pub archive_hash: String,
    pub file_size_mb: i64,
    pub install_instructions: String,
    pub dll_whitelist: Option<String>,
}

pub struct DbState(pub Mutex<Connection>);

// --- ИНИЦИАЛИЗАЦИЯ ---

pub fn init(app_data_dir: PathBuf) -> Result<Connection> {
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir).expect("Не удалось создать папку данных приложения");
    }

    let db_path = app_data_dir.join("launcher.db");
    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;"
    )?;

    // ОБНОВЛЕННЫЕ ТАБЛИЦЫ
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            image_url TEXT,
            install_path TEXT
        );

        CREATE TABLE IF NOT EXISTS localizations (
            id TEXT PRIMARY KEY,
            game_id TEXT NOT NULL,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            author TEXT,
            source_url TEXT,
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

// --- КОМАНДЫ ---

#[tauri::command]
pub fn get_games(state: State<DbState>) -> Result<Vec<Game>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, description, image_url, install_path FROM games")
        .map_err(|e| e.to_string())?;
    
    let games_iter = stmt.query_map([], |row| {
        Ok(Game {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            image_url: row.get(3)?,
            install_path: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut games = Vec::new();
    for game in games_iter { games.push(game.map_err(|e| e.to_string())?); }
    Ok(games)
}

#[tauri::command]
pub fn get_localizations(game_id: String, state: State<DbState>) -> Result<Vec<Localization>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT l.id, l.name, l.version, l.author, l.source_url, l.file_size_mb,
                    COALESCE(s.status, 'available') as status
             FROM localizations l
             LEFT JOIN install_states s ON l.id = s.localization_id
             WHERE l.game_id = ?1"
        )
        .map_err(|e| e.to_string())?;

    let locs_iter = stmt.query_map(params![game_id], |row| {
        Ok(Localization {
            id: row.get(0)?,
            name: row.get(1)?,
            version: row.get(2)?,
            author: row.get(3)?,
            source_url: row.get(4)?,
            file_size_mb: row.get(5)?,
            status: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut localizations = Vec::new();
    for loc in locs_iter { localizations.push(loc.map_err(|e| e.to_string())?); }
    Ok(localizations)
}

#[tauri::command]
pub fn sync_catalog(state: State<DbState>, json_string: String) -> Result<(), String> {
    let catalog: Vec<CatalogGame> = serde_json::from_str(&json_string).map_err(|e| format!("Ошибка парсинга JSON: {}", e))?;
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for game in catalog {
        tx.execute(
            "INSERT INTO games (id, name, description, image_url) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, description=excluded.description, image_url=excluded.image_url",
            params![game.id, game.name, game.description, game.image_url],
        ).map_err(|e| e.to_string())?;

        for loc in game.localizations {
            tx.execute(
                "INSERT INTO localizations (id, game_id, name, version, author, source_url, primary_url, backup_url, archive_hash, file_size_mb, install_instructions, dll_whitelist)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET 
                 name=excluded.name, version=excluded.version, author=excluded.author, source_url=excluded.source_url,
                 primary_url=excluded.primary_url, backup_url=excluded.backup_url, archive_hash=excluded.archive_hash,
                 file_size_mb=excluded.file_size_mb, install_instructions=excluded.install_instructions,
                 dll_whitelist=excluded.dll_whitelist",
                params![
                    loc.id, game.id, loc.name, loc.version, loc.author, loc.source_url,
                    loc.primary_url, loc.backup_url, loc.archive_hash, loc.file_size_mb, 
                    loc.install_instructions, loc.dll_whitelist
                ],
            ).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_game_path(game_id: String, state: State<DbState>) -> Result<String, String> {
    let folder = FileDialog::new().set_title("Выберите папку с установленной игрой").pick_folder();
    match folder {
        Some(path) => {
            let path_str = path.to_string_lossy().to_string();
            let conn = state.0.lock().map_err(|e| e.to_string())?;
            conn.execute("UPDATE games SET install_path = ?1 WHERE id = ?2", params![path_str, game_id]).map_err(|e| e.to_string())?;
            Ok(path_str)
        }
        None => Err("Выбор папки отменен".to_string())
    }
}

#[tauri::command]
pub fn reset_game_path(game_id: String, state: State<DbState>) -> Result<(), String>{
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE games SET install_path = NULL WHERE id = ?1",
        params![game_id],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

// #[tauri::command]
// pub fn start_installation(localization_id: String, state: State<DbState>) -> Result<String, String> {
//     println!("=== НАЧАЛАСЬ УСТАНОВКА ===");

//     let conn = state.0.lock().map_err(|e| e.to_string())?;

//     let game_id: String = conn.query_row(
//         "SELECT game_id FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0),
//     ).map_err(|e| format!("Перевод не найден: {}", e))?;
//     println!("1. ID игры получено: {}", game_id);

//     let install_path: Option<String> = conn.query_row(
//         "SELECT install_path FROM games WHERE id = ?1", params![game_id], |row| row.get(0),
//     ).map_err(|e| format!("Игра не найдена: {}", e))?;

//     let path = install_path.ok_or("Путь к игре не указан!".to_string())?;
//     println!("2. Путь к игре: {}", path);

//     let instructions: String = conn.query_row(
//         "SELECT install_instructions FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0),
//     ).map_err(|e| e.to_string())?;
//     println!("3. Инструкции: {}", instructions);

//     conn.execute(
//         "INSERT INTO install_states (localization_id, status, error_message) VALUES (?1, 'installing', NULL)
//          ON CONFLICT(localization_id) DO UPDATE SET status='installing', error_message=NULL",
//         params![localization_id],
//     ).map_err(|e| e.to_string())?;
//     println!("4. Статус в БД обновлен на 'installing'");

//     let fake_archive_path = "C:\\test_zip\\test.zip";
//     println!("5. Попытка распаковать ВРЕМЕННЫЙ архив: {}", fake_archive_path);
    
//     match crate::installer::extract_archive(fake_archive_path, &path, &instructions) {
//         Ok(_) => {
//             println!("6. РАСПАКОВКА УСПЕШНА!");
//             conn.execute(
//                 "UPDATE install_states SET status = 'installed', installed_version = '1.0' WHERE localization_id = ?1",
//                 params![localization_id],
//             ).map_err(|e| e.to_string())?;
//         }
//         Err(e) => {
//             println!("6. ОШИБКА РАСПАКОВКИ: {}", e);
//             conn.execute(
//                 "UPDATE install_states SET status = 'error', error_message = ?1 WHERE localization_id = ?2",
//                 params![e, localization_id],
//             ).map_err(|e| e.to_string())?;
//             return Err(e);
//         }
//     }

//     Ok(path)
// }

#[tauri::command]
pub async fn install_localization(
    localization_id: String,
    app: AppHandle, // Нужен для отправки событий (прогресса) в React
    state: State<'_, DbState>,
) -> Result<(), String> {
    
    // 1. БЛОКИРУЕМ БД, БЕРЕМ ДАННЫЕ
    let (game_id, install_path, primary_url, backup_url, instructions) = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;

        let game_id: String = conn.query_row(
            "SELECT game_id FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0),
        ).map_err(|e| format!("Перевод не найден: {}", e))?;

        let install_path: Option<String> = conn.query_row(
            "SELECT install_path FROM games WHERE id = ?1", params![game_id], |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        let path = install_path.ok_or("Путь к игре не указан!")?;

        let (p_url, b_url, instr): (String, Option<String>, String) = conn.query_row(
            "SELECT primary_url, backup_url, install_instructions FROM localizations WHERE id = ?1",
            params![localization_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        ).map_err(|e| e.to_string())?;

        // Меняем статус на 'downloading'
        conn.execute(
            "INSERT INTO install_states (localization_id, status) VALUES (?1, 'downloading')
             ON CONFLICT(localization_id) DO UPDATE SET status='downloading', error_message=NULL",
            params![localization_id],
        ).map_err(|e| e.to_string())?;

        // ВАЖНО: conn уничтожается здесь (выходит из области видимости). Mutex разблокируется!
        (game_id, path, p_url, b_url, instr)
    }; 

    // 2. СКАЧИВАНИЕ (БД РАЗБЛОКИРОВАНА, ЛАУНЧЕР НЕ ВИСИТ)
    let file_name = format!("{}.zip", localization_id);
    let archive_path = crate::downloader::download_with_fallback(
        app, 
        &primary_url, 
        backup_url.as_deref(), 
        &file_name
    ).await?;

    // 3. РАСПАКОВКА
    {
        // Снова кратко блокируем БД, чтобы поменять статус
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE install_states SET status = 'installing' WHERE localization_id = ?1",
            params![localization_id],
        ).map_err(|e| e.to_string())?;
    } // Mutex снова разблокирован

    // Вызываем наш экстрактор
    crate::installer::extract_archive(
        &archive_path.to_string_lossy(), 
        &install_path, 
        &instructions
    )?;

    // 4. ФИНАЛ
    {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE install_states SET status = 'installed' WHERE localization_id = ?1",
            params![localization_id],
        ).map_err(|e| e.to_string())?;
    }

    // Удаляем скачанный временный .zip архив за собой
    std::fs::remove_file(&archive_path).ok(); 

    Ok(())
}
