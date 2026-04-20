use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use serde::Deserialize;
use tauri::State;
use rfd::FileDialog;
use tauri::AppHandle;
use tauri::Manager;

// ============================================================================
// СТРУКТУРЫ ДЛЯ СЕРИАЛИЗАЦИИ (Передача данных из Rust во фронтенд)
// ============================================================================

/// Данные об игре, которые видит пользователь в интерфейсе.
#[derive(serde::Serialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub install_path: Option<String>,
}

/// Данные о конкретной локализации в интерфейсе.
#[derive(serde::Serialize)]
pub struct Localization {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub language: String,
    pub file_size_mb: i64,
    pub status: String, 
    pub is_managed: bool, // НОВОЕ: true, если перевод в Локальной Библиотеке
}

// ============================================================================
// СТРУКТУРЫ ДЛЯ ДЕСЕРИАЛИЗАЦИИ (Чтение JSON-каталога)
// ============================================================================

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
    pub name: String,
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

/// Глобальное состояние базы данных, защищенное Mutex для потокобезопасности.
pub struct DbState(pub Mutex<Connection>);

// ============================================================================
// ИНИЦИАЛИЗАЦИЯ БАЗЫ ДАННЫХ
// ============================================================================

/// Создает файл БД (если его нет), применяет настройки производительности
/// и создает таблицы по схеме.
pub fn init(app_data_dir: PathBuf) -> Result<Connection> {
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir).expect("CRITICAL: Не удалось создать папку данных приложения");
    }

    let db_path = app_data_dir.join("launcher.db");
    let conn = Connection::open(db_path)?;

    // WAL режим позволяет читать БД во время записи (избегает фризов UI).
    // Foreign Keys включаем для каскадного удаления зависимых данных.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;"
    )?;

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
            language TEXT DEFAULT 'Русский',
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
            local_archive_path TEXT, -- Путь к архиву в локальной библиотеке (%AppData%/library/)
            error_message TEXT,
            FOREIGN KEY (localization_id) REFERENCES localizations(id) ON DELETE CASCADE
        );
        "
    )?;

    Ok(conn)
}

// ============================================================================
// КОМАНДЫ ЧТЕНИЯ ДАННЫХ (Read)
// ============================================================================

/// Возвращает список всех игр, добавленных в лаунчер.
/// Возвращает список всех игр, добавленных в лаунчер.
/// Возвращает список всех игр, добавленных в лаунчер.
#[tauri::command]
pub fn get_games(state: State<DbState>) -> Result<Vec<Game>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, description, image_url, install_path FROM games")
        .map_err(|e| e.to_string())?;
    
    let mut games = Vec::new();
    
    // 1. Выполняем запрос и сразу забираем ошибку подготовки/выполнения 
    // у самого корня. `rows` становится чистым итератором.
    let mut rows = stmt.query_map([], |row| {
        Ok(Game {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            image_url: row.get(3)?,
            install_path: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    // 2. Явный цикл: забираем каждый ряд, обрабатываем ошибку парсинга ряда 
    // и кладем в вектор. Это полностью избавляет от конфликтов времени жизни.
    for row in rows {
        games.push(row.map_err(|e| e.to_string())?);
    }

    Ok(games)
}

/// Возвращает список локализаций для конкретной игры с их текущими статусами.
#[tauri::command]
pub fn get_localizations(game_id: String, state: State<DbState>) -> Result<Vec<Localization>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT l.id, l.name, l.version, l.author, l.source_url, l.language, l.file_size_mb,
                    COALESCE(s.status, 'available') as status,
                    CASE WHEN s.localization_id IS NOT NULL THEN 1 ELSE 0 END as is_managed
             FROM localizations l
             LEFT JOIN install_states s ON l.id = s.localization_id
             WHERE l.game_id = ?1"
        )
        .map_err(|e| e.to_string())?;

    let mut localizations = Vec::new(); // <--- ИСПРАВЛЕНО ЗДЕСЬ

    let mut rows = stmt.query_map(params![game_id], |row| {
        Ok(Localization {
            id: row.get(0)?,
            name: row.get(1)?,
            version: row.get(2)?,
            author: row.get(3)?,
            source_url: row.get(4)?,
            language: row.get(5)?,
            file_size_mb: row.get(6)?,
            status: row.get(7)?,
            is_managed: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    for row in rows {
        localizations.push(row.map_err(|e| e.to_string())?); // <--- ИСПРАВЛЕНО ЗДЕСЬ
    }

    Ok(localizations) // <--- ИСПРАВЛЕНО ЗДЕСЬ
}

// ============================================================================
// СИНХРОНИЗАЦИЯ И УПРАВЛЕНИЕ МЕТАДАННЫМИ
// ============================================================================

/// Забирает JSON-каталог с сервера и обновляет локальную БД.
/// Использует ON CONFLICT, чтобы не затирать локальные пути пользователей при обновлении описаний.
#[tauri::command]
pub fn sync_catalog(state: State<DbState>, json_string: String) -> Result<(), String> {
    let catalog: Vec<CatalogGame> = serde_json::from_str(&json_string).map_err(|e| format!("Ошибка парсинга JSON: {}", e))?;
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    
    // Транзакция гарантирует, что при ошибке парсинга одной игры БД не останется в полу-обновленном состоянии.
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

// ============================================================================
// РАБОТА С ПУТЯМИ ИГР
// ============================================================================

/// Открывает системный диалог для выбора папки с игрой и сохраняет путь в БД.
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

/// Сбрасывает путь к игре (устанавливает в NULL).
#[tauri::command]
pub fn reset_game_path(game_id: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE games SET install_path = NULL WHERE id = ?1", params![game_id]).map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// ЛОКАЛЬНОЕ ДОБАВЛЕНИЕ (БЕЗ СЕРВЕРА)
// ============================================================================

/// Создает игру в БД локально на основе пользовательского ввода.
#[tauri::command]
pub fn add_local_game(name: String, description: String, image_url: Option<String>, state: State<DbState>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    // Генерируем ID на основе названия (lowercase, пробелы -> подчеркивания)
    let id = name.to_lowercase().replace(" ", "_");
    
    conn.execute(
        "INSERT OR IGNORE INTO games (id, name, description, image_url) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, description, image_url],
    ).map_err(|e| e.to_string())?;

    Ok(id)
}

/// Открывает диалог выбора файла, вычисляет хэш/размер и сохраняет локализацию в БД.
#[tauri::command]
pub fn add_local_localization(
    game_id: String, name: String, version: String, language: String, 
    author: String, file_path: String, instructions_json: String, state: State<DbState>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let id = format!("{}_{}", game_id, name.to_lowercase().replace(" ", "_"));

    let hash = crate::installer::calculate_file_hash(&file_path)?;
    let metadata = std::fs::metadata(&file_path).map_err(|e| format!("Нет доступа к файлу: {}", e))?;
    let size_mb = (metadata.len() as f64 / 1_048_576.0) as i64;

    // Сохраняем локальный путь как primary_url (инсталлер сам поймет, что это локальный файл)
    conn.execute(
        "INSERT OR IGNORE INTO localizations (id, game_id, name, version, author, language, primary_url, archive_hash, file_size_mb, install_instructions)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![id, game_id, name, version, author, language, file_path, hash, size_mb, instructions_json],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Системный диалог для выбора только .zip архивов.
#[tauri::command]
pub fn pick_localization_file() -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("Архивы переводов", &["zip"])
        .set_title("Выберите архив с переводом")
        .pick_file();

    match file {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

// ============================================================================
// УСТАНОВКА, ВКЛЮЧЕНИЕ, ВЫКЛЮЧЕНИЕ, УДАЛЕНИЕ (Модификация файлов)
// ============================================================================

/// Основной пайплайн включения мода.
/// Скачивает (если нет в Library), проверяет хэш, бэкапит, распаковывает.
#[tauri::command]
pub async fn install_localization(
    localization_id: String,
    app: AppHandle, 
    state: State<'_, DbState>,
) -> Result<(), String> {
    
    // 1. Блокируем БД, забираем все нужные данные и СРАЗУ отпускаем Mutex.
    // Это критично для асинхронных функций, чтобы UI не зависал во время скачивания.
    let (game_id, install_path, primary_url, backup_url, instructions, expected_hash) = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;

        let game_id: String = conn.query_row("SELECT game_id FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0)).map_err(|e| format!("Перевод не найден: {}", e))?;
        let install_path: Option<String> = conn.query_row("SELECT install_path FROM games WHERE id = ?1", params![game_id], |row| row.get(0)).map_err(|e| e.to_string())?;
        let path = install_path.ok_or("Путь к игре не указан!")?;

        let (p_url, b_url, instr): (String, Option<String>, String) = conn.query_row(
            "SELECT primary_url, backup_url, install_instructions FROM localizations WHERE id = ?1",
            params![localization_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        ).map_err(|e| e.to_string())?;
        
        let hash: String = conn.query_row("SELECT archive_hash FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0)).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO install_states (localization_id, status) VALUES (?1, 'downloading')
             ON CONFLICT(localization_id) DO UPDATE SET status='downloading', error_message=NULL",
            params![localization_id],
        ).map_err(|e| e.to_string())?;

        (game_id, path, p_url, b_url, instr, hash)
    }; 

    // 2. Подготовка директорий Library и Backups
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let library_dir = app_dir.join("library");
    let backups_dir = app_dir.join("backups");
    std::fs::create_dir_all(&library_dir).map_err(|e| format!("Нет прав на создание папки библиотеки: {}", e))?;
    std::fs::create_dir_all(&backups_dir).map_err(|e| format!("Нет прав на создание папки бэкапов: {}", e))?;

    let final_archive_path = library_dir.join(format!("{}.zip", localization_id));

    // 3. Получение архива (Скачивание или копирование локального файла)
    if !primary_url.starts_with("http://") && !primary_url.starts_with("https://") {
        if !final_archive_path.exists() {
            std::fs::copy(&primary_url, &final_archive_path).map_err(|e| format!("Ошибка копирования: {}", e))?;
        }
    } else {
        if !final_archive_path.exists() {
            let temp_path = crate::downloader::download_with_fallback(app.clone(), &primary_url, backup_url.as_deref(), &format!("{}.zip", localization_id)).await?;
            std::fs::rename(&temp_path, &final_archive_path).map_err(|e| "Ошибка перемещения архива")?;
        }
    }

    // 4. Валидация безопасности: проверка SHA-256
    crate::installer::verify_file_hash(&final_archive_path.to_string_lossy(), &expected_hash)?;

    // 5. Создание бэкапа оригинальных файлов игры
    let backup_file_path = backups_dir.join(format!("{}.zip", localization_id));
    crate::installer::create_backup(&install_path, &instructions, &backup_file_path.to_string_lossy())?;

    // 6. Распаковка
    {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.execute("UPDATE install_states SET status = 'installing' WHERE localization_id = ?1", params![localization_id]).map_err(|e| e.to_string())?;
    }

    match crate::installer::extract_archive(&final_archive_path.to_string_lossy(), &install_path, &instructions) {
        Ok(_) => {}
        Err(e) => {
            let conn = state.0.lock().map_err(|e| e.to_string())?;
            conn.execute("UPDATE install_states SET status = 'error', error_message = ?1 WHERE localization_id = ?2", params![e, localization_id]).map_err(|e| e.to_string())?;
            return Err(e); 
        }
    }

    // 7. Финализация: сохраняем пути к бэкапу и архиву в БД
    {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE install_states SET status = 'installed', backup_path = ?1, local_archive_path = ?2 WHERE localization_id = ?3",
            params![backup_file_path.to_string_lossy(), final_archive_path.to_string_lossy(), localization_id],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Откатывает файлы игры из бэкапа, но оставляет архив в Library для быстрого включения.
#[tauri::command]
pub fn disable_localization(localization_id: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    // ИСПРАВЛЕНО: Делаем JOIN с таблицей localizations, чтобы получить game_id
    let (backup_path, game_id): (Option<String>, String) = conn.query_row(
        "SELECT s.backup_path, l.game_id 
         FROM install_states s
         JOIN localizations l ON s.localization_id = l.id
         WHERE s.localization_id = ?1",
        params![localization_id], |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| format!("Данные не найдены: {}", e))?;

    let install_path: String = conn.query_row("SELECT install_path FROM games WHERE id = ?1", params![game_id], |row| row.get(0)).map_err(|e| e.to_string())?;

    let backup = backup_path.ok_or("Нечего выключать: бэкап не найден.")?;
    crate::installer::restore_backup(&backup, &install_path)?;

    conn.execute(
        "UPDATE install_states SET status = 'available', backup_path = NULL WHERE localization_id = ?1",
        params![localization_id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Полное удаление локализации: откатывает файлы, удаляет бэкап и архив из Library.
#[tauri::command]
pub fn delete_localization(localization_id: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    // ИСПРАВЛЕНО: Делаем JOIN с таблицей localizations, чтобы получить game_id
    let (backup_path, local_archive_path, game_id): (Option<String>, Option<String>, String) = conn.query_row(
        "SELECT s.backup_path, s.local_archive_path, l.game_id 
         FROM install_states s
         JOIN localizations l ON s.localization_id = l.id
         WHERE s.localization_id = ?1",
        params![localization_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    ).map_err(|e| format!("Данные не найдены: {}", e))?;

    let install_path: String = conn.query_row("SELECT install_path FROM games WHERE id = ?1", params![game_id], |row| row.get(0)).map_err(|e| e.to_string())?;

    // 1. Откатываем файлы (если мод был включен)
    if let Some(backup) = backup_path {
        crate::installer::restore_backup(&backup, &install_path)?;
        std::fs::remove_file(&backup).ok(); 
    }

    // 2. Удаляем архив из локальной библиотеки навсегда
    if let Some(archive) = local_archive_path {
        std::fs::remove_file(&archive).ok();
        println!("[LIBRARY] Архив удален из библиотеки: {:?}", archive);
    }

    // 3. Удаляем запись из БД
    conn.execute("DELETE FROM install_states WHERE localization_id = ?1", params![localization_id]).map_err(|e| e.to_string())?;

    Ok(())
}