use std::fs::{self, File};
use std::path::PathBuf;
use zip::read::ZipArchive;
use zip::write::{FileOptions, ZipWriter};
use sha2::{Sha256, Digest};
use std::collections::HashSet;

// Правило маппинга из manifest/install_instructions:
// `src` — путь внутри архива, `dest` — целевая папка/файл внутри игры.
#[derive(serde::Deserialize)]
struct MappingInstruction {
    src: String,
    dest: String,
}

/// Распаковывает архив перевода в папку игры.
/// Поведение зависит от `instructions_json`:
/// - пустой список: распаковка всех файлов в корень игры;
/// - непустой список: копирование только файлов, совпавших с правилами `src -> dest`.
pub fn extract_archive(
    archive_path: &str,
    target_dir: &str,
    instructions_json: &str,
) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|e| format!("Не удалось открыть архив: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Архив повреждён: {}", e))?;

    // Некорректные инструкции считаем пустыми, чтобы не блокировать установку в legacy-кейсах.
    let instructions: Vec<MappingInstruction> = 
        serde_json::from_str(instructions_json).unwrap_or_default();

    // Если инструкций нет, раскладываем архив "как есть".
    if instructions.is_empty() {
        println!("[INSTALLER] Инструкций нет. Распаковываю весь архив в корень: {}", target_dir);
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

            if file.is_file() {
                let file_name = file.name().to_string();
                let final_path = PathBuf::from(target_dir).join(&file_name);

                // Zip-Slip защита: нельзя выйти за пределы целевой директории игры.
                if !final_path.starts_with(target_dir) {
                    return Err(format!("ЗАБЛОКИРОВАНО БЕЗОПАСНОСТЬЮ: Попытка выйти за пределы папки игры: {}", file_name));
                }

                if let Some(parent) = final_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("Нет прав на создание папки: {}", e))?;
                }

                let mut out_file = File::create(&final_path).map_err(|e| format!("Нет прав на запись файла: {}", e))?;
                std::io::copy(&mut file, &mut out_file).map_err(|e| format!("Ошибка записи на диск: {}", e))?;
            }
        }
        return Ok(());
    }

    // При наличии инструкций распаковываем только разрешенные пути из манифеста.
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

        if file.is_file() {
            let file_name = file.name().to_string();
            println!("[INSTALLER] Найден файл в архиве: {}", file_name);

            for instr in &instructions {
                println!("[INSTALLER] Сверяю с правилом src='{}'", instr.src);

                if file_name.starts_with(&instr.src) {
                    let relative_path = file_name.strip_prefix(&instr.src).unwrap_or(&file_name);
                    let final_path = PathBuf::from(target_dir).join(&instr.dest).join(relative_path);

                    println!("[INSTALLER] СОВПАДЕНИЕ! Копирую в: {:?}", final_path);

                    // Дополнительная Zip-Slip защита для вычисленного пути по правилам.
                    if !final_path.starts_with(target_dir) {
                        return Err("ЗАБЛОКИРОВАНО БЕЗОПАСНОСТЬЮ: Правило пытается выйти за пределы папки игры.".to_string());
                    }

                    if let Some(parent) = final_path.parent() {
                        fs::create_dir_all(parent).map_err(|e| format!("Нет прав на создание папки: {}", e))?;
                    }

                    let mut out_file = File::create(&final_path).map_err(|e| format!("Нет прав на запись файла: {}", e))?;
                    std::io::copy(&mut file, &mut out_file).map_err(|e| format!("Ошибка записи на диск: {}", e))?;

                    break;
                }
            }
        }
    }

    Ok(())
}

/// Создает zip-бэкап текущих файлов игры, которые потенциально будут затронуты установкой.
/// Бэкап нужен для `disable_localization` и `delete_localization`.
pub fn create_backup(
    game_path: &str,
    instructions_json: &str,
    backup_archive_path: &str,
) -> Result<(), String> {
    let instructions: Vec<MappingInstruction> = 
        serde_json::from_str(instructions_json)
        .map_err(|e| format!("Ошибка парсинга инструкций для бэкапа: {}", e))?;
    
    let file = File::create(backup_archive_path).map_err(|e| format!("Не удалось создать файл бэкапа: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut backed_up_files_count = 0;

    for instr in &instructions {
        // Строим конечный путь относительно папки игры.
        let target_dir = PathBuf::from(game_path).join(&instr.dest);

        // Если правило указывает на конкретный файл — кладем его в бэкап отдельной записью.
        if target_dir.exists() && !target_dir.is_dir() {
            let file_name = target_dir.file_name().unwrap().to_string_lossy();
            zip.start_file(file_name.to_string(), options.clone()).map_err(|e| e.to_string())?;
            let mut f = File::open(&target_dir).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
            backed_up_files_count += 1;

        }
        else {
            // Если правило указывает на директорию — архивируем файлы первого уровня в этой директории.
            if target_dir.is_dir() {
                for entry in fs::read_dir(&target_dir).map_err(|e| e.to_string())?{
                    let entry = entry.map_err(|e| e.to_string())?;
                    let path = entry.path();

                    if path.is_file() {
                        // Сохраняем относительную структуру путей внутри backup-архива.
                        let relative = path.strip_prefix(game_path).unwrap().to_string_lossy();
                        zip.start_file(relative.to_string(), options.clone()).map_err(|e| e.to_string())?;
                        let mut f = File::open(&path).map_err(|e| e.to_string())?;
                        std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
                        backed_up_files_count += 1;
                    }
                }
            }
        }

    }
    zip.finish().map_err(|e| format!("Ошибка завершения архива бэкапа: {}", e))?;
    println!("[BACKUP] Создан бэкап {} файлов по пути: {}", backed_up_files_count, backup_archive_path);
    Ok(())
}

/// Восстанавливает файлы игры из backup-архива.
pub fn restore_backup(
    backup_path: &str,
    game_path: &str
) -> Result<(), String> {
    let file = File::open(backup_path)
        .map_err(|e| format!("Не удалось открыть архив бэкапа: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("архив бэкапа повреждён: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

        if file.is_file() {
            let file_name = file.name().to_string();

            let final_path = PathBuf::from(game_path).join(&file_name);

            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("Нет прав на создание папки: {}", e))?;
            }
            
            let mut out_file = File::create(&final_path).map_err(|e| format!("нет прав на запись файла: {}", e))?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| format!("Ошибка записи на диск: {}", e))?;
        }
    }
    println!("[BACKUP] Бэкап успешно восстановлен из: {}", backup_path);
    Ok(())
}

/// Проверяет SHA-256 файла относительно эталона из БД.
/// При несовпадении установка должна быть прервана.
pub fn verify_file_hash(file_path: &str, expected_hash: &str) -> Result<(), String> {
    let actual_hash = calculate_file_hash(file_path)?;

    // Для стабильного сравнения приводим эталон к lower-case.
    if actual_hash != expected_hash.to_lowercase() {
        return Err(format!(
            "ОШИБКА БЕЗОПАСНОСТИ: Хэш файла не совпадает! Ожидался: {}, Получен: {}. Файл был подменен или поврежден.",
            expected_hash, actual_hash
        ));
    }

    println!("[SECURITY] Хэш файла успешно проверен.");
    Ok(())
}

/// Вычисляет SHA-256 файла и возвращает строку (для сохранения в БД)
pub fn calculate_file_hash(file_path: &str) -> Result<String, String> {
    let mut file = std::fs::File::open(file_path)
        .map_err(|e| format!("Не удалось открыть файл: {}", e))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = std::io::Read::read(&mut file, &mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 { break; }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    
    // Преобразуем digest в hex без дополнительных зависимостей.
    let hex_string: String = result.iter().map(|b| format!("{:02x}", b)).collect();
    
    Ok(hex_string)
}

/// Вычисляет множество путей файлов, которые будут затронуты установкой мода.
/// Используется для проверки конфликтов между несколькими активными модами.
pub fn get_mod_target_paths(archive_path: &str, instructions_json: &str) -> Result<HashSet<String>, String> {
    let instructions: Vec<MappingInstruction> = serde_json::from_str(instructions_json)
        .map_err(|e| format!("Ошибка парсинга инструкций: {}", e))?;

    let file = File::open(archive_path)
        .map_err(|e| format!("Не удалось открыть архив для проверки конфликтов: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Архив поврежден: {}", e))?;

    let mut target_paths = HashSet::new();

    // Пустые инструкции = "распаковать как есть в корень игры".
    if instructions.is_empty() {
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| e.to_string())?;
            if !file.is_file() {
                continue;
            }

            target_paths.insert(normalize_rel_path(file.name()));
        }
        return Ok(target_paths);
    }

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        if !file.is_file() { continue; }

        let file_name = file.name().to_string();

        for instr in &instructions {
            if file_name.starts_with(&instr.src) {
                let relative_path = file_name.strip_prefix(&instr.src).unwrap_or(&file_name);
                let final_path = std::path::PathBuf::from(&instr.dest).join(relative_path);
                target_paths.insert(normalize_rel_path(&final_path.to_string_lossy()));
                break;
            }
        }
    }

    Ok(target_paths)
}


/// Проверяет, не конфликтует ли новый мод с уже установленными модами.
/// Возвращает `Some(String)`, если есть конфликт, и `None`, если всё чисто.
pub fn check_conflicts(new_paths: &HashSet<String>, active_paths: &[String]) -> Option<String> {
    let active_set: HashSet<&str> = active_paths.iter().map(|p| p.as_str()).collect();

    for target_path in new_paths {
        if active_set.contains(target_path.as_str()) {
            return Some(format!(
                "Конфликт файлов: файл `{}` уже изменен другим активным переводом. Выключите его перед установкой.",
                target_path
            ));
        }
    }

    None
}

// Единый формат путей для сравнения конфликтов (lower-case + unix slashes).
fn normalize_rel_path(path: &str) -> String {
    path
        .replace("\\", "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_zip(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let temp = tempfile::NamedTempFile::new().expect("tempfile should be created");
        let file = std::fs::File::create(temp.path()).expect("zip file should be created");
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, content) in entries {
            zip.start_file(*name, options).expect("zip entry should be started");
            zip.write_all(content).expect("zip entry should be written");
        }

        zip.finish().expect("zip should be finalized");
        temp
    }

    #[test]
    fn check_conflicts_detects_same_file() {
        let new_paths = HashSet::from([String::from("data/shared.pak")]);
        let active_paths = vec![String::from("data/shared.pak")];

        let result = check_conflicts(&new_paths, &active_paths);
        assert!(result.is_some());
    }

    #[test]
    fn check_conflicts_ignores_different_files() {
        let new_paths = HashSet::from([String::from("data/new_file.pak")]);
        let active_paths = vec![String::from("data/old_file.pak")];

        let result = check_conflicts(&new_paths, &active_paths);
        assert!(result.is_none());
    }

    #[test]
    fn get_mod_target_paths_handles_empty_instructions() {
        let archive = create_zip(&[
            ("Data/Shared.pak", b"123"),
            ("readme.txt", b"abc"),
        ]);

        let paths = get_mod_target_paths(archive.path().to_str().unwrap(), "[]")
            .expect("target paths should be calculated");

        assert!(paths.contains("data/shared.pak"));
        assert!(paths.contains("readme.txt"));
    }
}
