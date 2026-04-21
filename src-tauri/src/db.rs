use rusqlite::{params, Connection, Result};
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::Deserialize;
use tauri::State;
use rfd::FileDialog;
use tauri::AppHandle;
use tauri::Manager;

#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
#[cfg(target_os = "windows")]
use winreg::{HKEY, RegKey};

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

#[derive(Debug, Clone)]
struct GameInstallCandidate {
    display_name: String,
    install_path: PathBuf,
    source: &'static str,
}

#[derive(Deserialize)]
struct EpicManifest {
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "InstallLocation")]
    install_location: Option<String>,
    #[serde(rename = "AppName")]
    app_name: Option<String>,
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
#[tauri::command]
pub fn get_games(state: State<DbState>) -> Result<Vec<Game>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, description, image_url, install_path FROM games")
        .map_err(|e| e.to_string())?;
    
    let mut games = Vec::new();
    
    // Сначала получаем итератор строк из rusqlite.
    let mut rows = stmt.query_map([], |row| {
        Ok(Game {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            image_url: row.get(3)?,
            install_path: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    // Явный цикл упрощает обработку ошибок каждой строки и не упирается в lifetime-ловушки.
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

    let mut localizations = Vec::new();

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
        localizations.push(row.map_err(|e| e.to_string())?);
    }

    Ok(localizations)
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
    
    // Транзакция гарантирует атомарный апдейт каталога.
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

/// Автоматически ищет путь игры в Steam/Epic и сохраняет его в БД.
#[tauri::command]
pub fn auto_detect_game_path(game_id: String, state: State<DbState>) -> Result<String, String> {
    // 1) Получаем canonical-имя игры из нашей БД.
    let game_name: String = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT name FROM games WHERE id = ?1", params![game_id], |row| row.get(0))
            .map_err(|e| format!("Игра не найдена: {}", e))?
    };

    // 2) Строим список установленных игр из Steam/Epic.
    let candidates = collect_install_candidates();
    if candidates.is_empty() {
        return Err(
            "Автопоиск не нашел установленных игр в Steam/Epic. Укажите путь вручную.".to_string()
        );
    }

    // 3) Выбираем лучшее совпадение по score-модели.
    let best = pick_best_candidate(&game_name, &candidates).ok_or_else(|| {
        format!(
            "Автопоиск не смог сопоставить `{}` с установленными играми. Укажите путь вручную.",
            game_name
        )
    })?;

    // 4) Фиксируем путь в БД и возвращаем его во frontend.
    let detected_path = best.install_path.to_string_lossy().to_string();
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE games SET install_path = ?1 WHERE id = ?2",
        params![detected_path, game_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(detected_path)
}

fn collect_install_candidates() -> Vec<GameInstallCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(collect_steam_candidates());
    candidates.extend(collect_epic_candidates());

    // Убираем дубликаты, если один и тот же путь встретился из разных источников.
    let mut seen_paths = HashSet::new();
    let mut deduped = Vec::new();
    for candidate in candidates {
        let key = candidate
            .install_path
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        if seen_paths.insert(key) {
            deduped.push(candidate);
        }
    }

    deduped
}

fn pick_best_candidate<'a>(
    game_name: &str,
    candidates: &'a [GameInstallCandidate],
) -> Option<&'a GameInstallCandidate> {
    let mut best_score = 0;
    let mut best_candidate = None;

    for candidate in candidates {
        let score = score_candidate(game_name, candidate);
        if score > best_score {
            best_score = score;
            best_candidate = Some(candidate);
        }
    }

    // Ниже порога слишком высокий риск ложного срабатывания.
    if best_score < 65 {
        return None;
    }

    best_candidate
}

fn score_candidate(game_name: &str, candidate: &GameInstallCandidate) -> i32 {
    // Сравниваем и display_name из манифеста, и имя папки установки.
    // Иногда один из этих источников более точный, чем другой.
    let name_score = score_match_text(game_name, &candidate.display_name);
    let folder_name = candidate
        .install_path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder_score = score_match_text(game_name, &folder_name) + 5;

    let mut best = name_score.max(folder_score);
    if candidate.source == "Steam" || candidate.source == "Epic" {
        best += 1;
    }
    best
}

fn score_match_text(target_raw: &str, candidate_raw: &str) -> i32 {
    let target = normalize_text(target_raw);
    let candidate = normalize_text(candidate_raw);

    if target.is_empty() || candidate.is_empty() {
        return 0;
    }

    if target == candidate {
        return 120;
    }

    if target.len() >= 4 && candidate.len() >= 4 && (target.contains(&candidate) || candidate.contains(&target))
    {
        return 100;
    }

    // Токен-метрика покрывает кейсы с пунктуацией, разным регистром и лишними словами.
    let target_tokens = tokenize(target_raw);
    let candidate_tokens = tokenize(candidate_raw);
    if target_tokens.is_empty() || candidate_tokens.is_empty() {
        return 0;
    }

    let common = target_tokens.intersection(&candidate_tokens).count();
    if common == 0 {
        return 0;
    }

    let coverage = common as f64 / target_tokens.len() as f64;
    if coverage >= 1.0 {
        95
    } else if coverage >= 0.6 {
        80
    } else if coverage >= 0.4 {
        65
    } else if coverage >= 0.25 {
        50
    } else {
        0
    }
}

fn normalize_text(value: &str) -> String {
    // Нормализация для fast-сравнения (без пробелов, знаков и регистра).
    value
        .chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn tokenize(value: &str) -> HashSet<String> {
    // Выделяем "значимые" куски имени игры, исключая разделители.
    value
        .split(|c: char| !c.is_alphanumeric())
        .map(|part| part.trim().to_lowercase())
        .filter(|part| part.len() >= 2)
        .collect()
}

fn collect_steam_candidates() -> Vec<GameInstallCandidate> {
    #[cfg(not(target_os = "windows"))]
    {
        return Vec::new();
    }

    #[cfg(target_os = "windows")]
    {
        // Сначала определяем корневую папку Steam, затем обходим все library folders.
        let Some(steam_root) = detect_steam_root() else {
            return Vec::new();
        };

        let mut candidates = Vec::new();
        for library in steam_library_paths(&steam_root) {
            let manifests_dir = library.join("steamapps");
            let entries = match std::fs::read_dir(&manifests_dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = path
                    .file_name()
                    .map(|v| v.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if !file_name.starts_with("appmanifest_") || !file_name.ends_with(".acf") {
                    continue;
                }

                // Парсим appmanifest_*.acf минимально необходимыми полями.
                let manifest = match std::fs::read_to_string(&path) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let Some(installdir) = vdf_value(&manifest, "installdir") else {
                    continue;
                };
                let display_name = vdf_value(&manifest, "name").unwrap_or_else(|| installdir.clone());
                let install_path = manifests_dir.join("common").join(installdir);
                if !install_path.exists() {
                    continue;
                }

                candidates.push(GameInstallCandidate {
                    display_name,
                    install_path,
                    source: "Steam",
                });
            }
        }

        candidates
    }
}

fn collect_epic_candidates() -> Vec<GameInstallCandidate> {
    #[cfg(not(target_os = "windows"))]
    {
        return Vec::new();
    }

    #[cfg(target_os = "windows")]
    {
        // Epic хранит список установленных игр в JSON-файлах `.item`.
        let Some(manifests_dir) = detect_epic_manifests_dir() else {
            return Vec::new();
        };
        let entries = match std::fs::read_dir(&manifests_dir) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        let mut candidates = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|v| v.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !file_name.ends_with(".item") {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let manifest: EpicManifest = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let Some(raw_install_path) = manifest.install_location else {
                continue;
            };
            let install_path = PathBuf::from(raw_install_path);
            if !install_path.exists() {
                continue;
            }

            let display_name = manifest
                .display_name
                .or(manifest.app_name)
                .unwrap_or_else(|| "Epic Game".to_string());

            candidates.push(GameInstallCandidate {
                display_name,
                install_path,
                source: "Epic",
            });
        }

        candidates
    }
}

#[cfg(target_os = "windows")]
fn detect_steam_root() -> Option<PathBuf> {
    // Основные источники: HKCU, HKLM (WOW6432Node), затем fallback по Program Files.
    for (hive, key, value) in [
        (HKEY_CURRENT_USER, "Software\\Valve\\Steam", "SteamPath"),
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath"),
    ] {
        if let Some(path) = read_registry_path(hive, key, value) {
            if path.exists() {
                return Some(path);
            }
        }
    }

    for env_name in ["PROGRAMFILES(X86)", "PROGRAMFILES"] {
        if let Ok(base) = env::var(env_name) {
            let candidate = PathBuf::from(base).join("Steam");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn steam_library_paths(steam_root: &Path) -> Vec<PathBuf> {
    // Первая библиотека всегда корень Steam, дополнительные — в libraryfolders.vdf.
    let mut libraries = vec![steam_root.to_path_buf()];
    let mut seen = HashSet::new();
    seen.insert(steam_root.to_string_lossy().to_lowercase());

    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    let content = match std::fs::read_to_string(vdf_path) {
        Ok(v) => v,
        Err(_) => return libraries,
    };

    for line in content.lines() {
        let Some((key, raw_path)) = parse_vdf_kv_line(line) else {
            continue;
        };
        if !key.eq_ignore_ascii_case("path") {
            continue;
        }

        let library_path = PathBuf::from(raw_path.replace("\\\\", "\\"));
        let normalized = library_path.to_string_lossy().to_lowercase();
        if library_path.exists() && seen.insert(normalized) {
            libraries.push(library_path);
        }
    }

    libraries
}

#[cfg(target_os = "windows")]
fn detect_epic_manifests_dir() -> Option<PathBuf> {
    // В новых установках берем AppDataPath из реестра, иначе fallback в ProgramData.
    if let Some(app_data_path) = read_registry_path(
        HKEY_LOCAL_MACHINE,
        "SOFTWARE\\WOW6432Node\\Epic Games\\EpicGamesLauncher",
        "AppDataPath",
    ) {
        let candidate = app_data_path.join("Manifests");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let base = env::var("PROGRAMDATA").ok()?;
    let fallback = PathBuf::from(base)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    if fallback.exists() {
        Some(fallback)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn read_registry_path(hive: HKEY, subkey: &str, value_name: &str) -> Option<PathBuf> {
    // Небольшой helper, чтобы не дублировать boilerplate winreg-кода.
    let root = RegKey::predef(hive);
    let key = root.open_subkey(subkey).ok()?;
    let value: String = key.get_value(value_name).ok()?;
    let normalized = value.replace('/', "\\");
    Some(PathBuf::from(normalized))
}

#[cfg(target_os = "windows")]
fn parse_vdf_kv_line(line: &str) -> Option<(String, String)> {
    // Упрощенный парсер строки формата `"key"   "value"` из VDF.
    let parts: Vec<&str> = line.split('"').collect();
    if parts.len() < 4 {
        return None;
    }

    let key = parts[1].trim();
    let value = parts[3].trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }

    Some((key.to_string(), value.to_string()))
}

fn vdf_value(content: &str, wanted_key: &str) -> Option<String> {
    // Читаем только плоские key/value строки — достаточно для appmanifest-полей name/installdir.
    for line in content.lines() {
        #[cfg(target_os = "windows")]
        {
            let Some((key, value)) = parse_vdf_kv_line(line) else {
                continue;
            };
            if key.eq_ignore_ascii_case(wanted_key) {
                return Some(value.replace("\\\\", "\\"));
            }
        }
    }
    None
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

    // Защита от дубликатов
    let count: i32 = conn.query_row(
        "SELECT count(*) FROM localizations WHERE id = ?1", 
        params![id], 
        |row| row.get(0) // Указываем, что хотим забрать нулевую колонку как i32
    ).map_err(|e| e.to_string())?;

    // Если count больше 0, значит дубликат
    if count > 0 {
        return Err("Перевод с таким названием уже существует.".to_string());
    }

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
    let (game_id, install_path, primary_url, backup_url, instructions, expected_hash, file_size_mb) = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;

        let game_id: String = conn.query_row("SELECT game_id FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0)).map_err(|e| format!("Перевод не найден: {}", e))?;
        let install_path: Option<String> = conn.query_row("SELECT install_path FROM games WHERE id = ?1", params![game_id], |row| row.get(0)).map_err(|e| e.to_string())?;
        let path = install_path.ok_or("Путь к игре не указан!")?;

        let (p_url, b_url, instr, size_mb): (String, Option<String>, String, Option<i64>) = conn.query_row(
            "SELECT primary_url, backup_url, install_instructions, file_size_mb FROM localizations WHERE id = ?1",
            params![localization_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        ).map_err(|e| e.to_string())?;
        
        let hash: String = conn.query_row("SELECT archive_hash FROM localizations WHERE id = ?1", params![localization_id], |row| row.get(0)).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO install_states (localization_id, status) VALUES (?1, 'downloading')
             ON CONFLICT(localization_id) DO UPDATE SET status='downloading', error_message=NULL",
            params![localization_id],
        ).map_err(|e| e.to_string())?;

        (game_id, path, p_url, b_url, instr, hash, size_mb)
    }; 

    // 2. Подготовка директорий Library и Backups
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let library_dir = app_dir.join("library");
    let backups_dir = app_dir.join("backups");
    std::fs::create_dir_all(&library_dir).map_err(|e| format!("Нет прав на создание папки библиотеки: {}", e))?;
    std::fs::create_dir_all(&backups_dir).map_err(|e| format!("Нет прав на создание папки бэкапов: {}", e))?;

    let final_archive_path = library_dir.join(format!("{}.zip", localization_id));
    let archive_size_hint = resolve_archive_size_hint(&final_archive_path, &primary_url, file_size_mb)
        .map_err(|e| format!("Не удалось оценить размер архива: {}", e))?;

    // 3. Получение архива (Скачивание или копирование локального файла)
    if !final_archive_path.exists() {
        let required_for_download = archive_size_hint.saturating_add(64 * 1024 * 1024);
        if let Err(e) = ensure_disk_space(
            &library_dir,
            required_for_download,
            "перед скачиванием/копированием архива в локальную библиотеку",
        ) {
            mark_install_error(&state, &localization_id, &e)?;
            return Err(e);
        }
    }

    if !primary_url.starts_with("http://") && !primary_url.starts_with("https://") {
        if !final_archive_path.exists() {
            std::fs::copy(&primary_url, &final_archive_path).map_err(|e| format!("Ошибка копирования: {}", e))?;
        }
    } else {
        if !final_archive_path.exists() {
            let temp_path = crate::downloader::download_with_fallback(app.clone(), &primary_url, backup_url.as_deref(), &format!("{}.zip", localization_id)).await?;
            std::fs::rename(&temp_path, &final_archive_path).map_err(|_| "Ошибка перемещения архива".to_string())?;
        }
    }

    let archive_size = std::fs::metadata(&final_archive_path)
        .map(|m| m.len())
        .unwrap_or(archive_size_hint);

    let required_for_backup = archive_size.saturating_add(64 * 1024 * 1024);
    if let Err(e) = ensure_disk_space(
        &backups_dir,
        required_for_backup,
        "перед созданием бэкапа оригинальных файлов",
    ) {
        mark_install_error(&state, &localization_id, &e)?;
        return Err(e);
    }

    // Бэкап + распаковка обычно требуют значительно больше места, чем размер архива.
    let required_for_extract = archive_size
        .saturating_mul(2)
        .saturating_add(256 * 1024 * 1024);
    if let Err(e) = ensure_disk_space(
        Path::new(&install_path),
        required_for_extract,
        "перед распаковкой перевода в папку игры",
    ) {
        mark_install_error(&state, &localization_id, &e)?;
        return Err(e);
    }

    // 3.5. ПРОВЕРКА КОНФЛИКТОВ (до бэкапа!)
    let active_rules = crate::db::get_active_rules(game_id.clone(), localization_id.clone(), state.clone())?;
    let new_paths = crate::installer::get_mod_target_paths(&final_archive_path.to_string_lossy(), &instructions)?;
    
    if let Some(conflict_msg) = crate::installer::check_conflicts(&new_paths, &active_rules) {
    mark_install_error(&state, &localization_id, &conflict_msg)?;
    return Err(conflict_msg); // Теперь это String!
    }

    // 4. Валидация безопасности: проверка SHA-256
    if let Err(e) = crate::installer::verify_file_hash(&final_archive_path.to_string_lossy(), &expected_hash) {
        mark_install_error(&state, &localization_id, &e)?;
        return Err(e);
    }

    // 5. Создание бэкапа оригинальных файлов игры
    let backup_file_path = backups_dir.join(format!("{}.zip", localization_id));
    if let Err(e) = crate::installer::create_backup(&install_path, &instructions, &backup_file_path.to_string_lossy()) {
        mark_install_error(&state, &localization_id, &e)?;
        return Err(e);
    }

    // 6. Распаковка
    {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.execute("UPDATE install_states SET status = 'installing' WHERE localization_id = ?1", params![localization_id]).map_err(|e| e.to_string())?;
    }

    match crate::installer::extract_archive(&final_archive_path.to_string_lossy(), &install_path, &instructions) {
        Ok(_) => {}
        Err(e) => {
            mark_install_error(&state, &localization_id, &e)?;
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

fn resolve_archive_size_hint(
    final_archive_path: &Path,
    primary_url: &str,
    file_size_mb: Option<i64>,
) -> Result<u64, String> {
    if final_archive_path.exists() {
        let size = std::fs::metadata(final_archive_path)
            .map_err(|e| format!("Не удалось получить размер архива в библиотеке: {}", e))?
            .len();
        return Ok(size);
    }

    if primary_url.starts_with("http://") || primary_url.starts_with("https://") {
        let mb = file_size_mb.unwrap_or(0).max(0) as u64;
        return Ok(mb.saturating_mul(1_048_576));
    }

    let size = std::fs::metadata(primary_url)
        .map_err(|e| format!("Не удалось получить размер локального архива: {}", e))?
        .len();
    Ok(size)
}

fn ensure_disk_space(path: &Path, required_bytes: u64, stage: &str) -> Result<(), String> {
    if required_bytes == 0 {
        return Ok(());
    }

    let available_bytes = fs2::available_space(path).map_err(|e| {
        format!(
            "Не удалось проверить свободное место на диске для `{}`: {}",
            path.display(),
            e
        )
    })?;

    if available_bytes < required_bytes {
        return Err(format!(
            "Недостаточно места на диске {}. Нужно примерно {}, доступно {}.",
            stage,
            format_size(required_bytes),
            format_size(available_bytes),
        ));
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    let gib = bytes as f64 / 1_073_741_824.0;
    if gib >= 1.0 {
        return format!("{:.2} GB", gib);
    }

    let mib = bytes as f64 / 1_048_576.0;
    format!("{:.0} MB", mib)
}

fn mark_install_error(state: &State<'_, DbState>, localization_id: &str, error_message: &str) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE install_states SET status = 'error', error_message = ?1 WHERE localization_id = ?2",
        params![error_message, localization_id],
    ).map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn get_active_rules(
    game_id: String,
    exclude_localization_id: String,
    state: tauri::State<'_, DbState>,
) -> Result<Vec<String>, String> {
    let installed_mods: Vec<(Option<String>, String, String)> = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT s.local_archive_path, l.primary_url, l.install_instructions
                 FROM localizations l
                 JOIN install_states s ON l.id = s.localization_id
                 WHERE l.game_id = ?1 AND s.status = 'installed' AND l.id != ?2",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![game_id, exclude_localization_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| e.to_string())?;

        let mut mods = Vec::new();
        for row in rows {
            mods.push(row.map_err(|e| e.to_string())?);
        }
        mods
    };

    let mut active_paths: HashSet<String> = HashSet::new();
    for (local_archive_path, primary_url, instructions) in installed_mods {
        let archive_path = local_archive_path.unwrap_or(primary_url);

        if archive_path.starts_with("http://") || archive_path.starts_with("https://") {
            return Err("Невозможно проверить конфликт: архив активного перевода не сохранен локально.".to_string());
        }

        if !std::path::Path::new(&archive_path).exists() {
            return Err(format!(
                "Невозможно проверить конфликт: не найден архив активного перевода `{}`.",
                archive_path
            ));
        }

        let paths = crate::installer::get_mod_target_paths(&archive_path, &instructions)?;
        active_paths.extend(paths);
    }

    Ok(active_paths.into_iter().collect())
}
