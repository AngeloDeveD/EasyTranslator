use std::fs::{self, File};
use std::path::PathBuf;
use zip::read::ZipArchive;
use zip::write::{FileOptions, ZipWriter};
use sha2::{Sha256, Digest};

// Вспомогательная структура для парсинга JSON
#[derive(serde::Deserialize)]
struct MappingInstruction {
    src: String,
    dest: String,
}

//Функция распаковки архива
pub fn extract_archive(
    archive_path: &str,
    target_dir: &str,
    instructions_json: &str,
) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|e| format!("Не удалось открыть архив: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Архив повреждён: {}", e))?;

    // Парсим инструкции
    let instructions: Vec<MappingInstruction> = 
        serde_json::from_str(instructions_json).unwrap_or_default();

    // ЕСЛИ ИНСТРУКЦИЙ НЕТ -> ПРОСТО РАСПАКОВЫВАЕМ ВСЁ В КОРЕНЬ ИГРЫ
    if instructions.is_empty() {
        println!("[INSTALLER] Инструкций нет. Распаковываю весь архив в корень: {}", target_dir);
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

            if file.is_file() {
                let file_name = file.name().to_string();
                let final_path = PathBuf::from(target_dir).join(&file_name);

                // БЕЗОПАСНОСТЬ (Защита от Zip-Slip): проверяем, чтобы файл не вышел за пределы папки игры
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

    // ЕСЛИ ИНСТРУКЦИИ ЕСТЬ -> РАБОТАЕМ ПО НИМ (Старая логика)
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

                    // Защита от Zip-Slip
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

//Функция для создании бэкапа
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
        //Узнаем полный путь к файлу в папке игры
        let target_dir = PathBuf::from(game_path).join(&instr.dest);

        //Если в правилен указан корректный файл (какой нибудь .dll)
        if target_dir.exists() && !target_dir.is_dir() {
            let file_name = target_dir.file_name().unwrap().to_string_lossy();
            zip.start_file(file_name.to_string(), options.clone()).map_err(|e| e.to_string())?;
            let mut f = File::open(&target_dir).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
            backed_up_files_count += 1;

        }
        else {
            if target_dir.is_dir() {
                for entry in fs::read_dir(&target_dir).map_err(|e| e.to_string())?{
                    let entry = entry.map_err(|e| e.to_string())?;
                    let path = entry.path();

                    if path.is_file() {
                        //Сохранение структуры папков внутри архива
                        let relative = path.strip_prefix(game_path).unwrap().to_string_lossy();
                        zip.start_file(relative.to_string(), options.clone()).map_err(|e| e.to_string())?;
                        let mut f = File::open(&path).map_err(|e| e.to_string())?;
                        std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
                        backed_up_files_count += 1;
                    }
                }
            }
        }

        //zip.finish().map_err(|e| format!("Ошибка завершения архива бэкапа: {}", e))?;
    }
    println!("[BACKUP] Создан бэкап {} файлов по пути: {}", backed_up_files_count, backup_archive_path);
    Ok(())
}

//Восстановление оригинальных файлов с бэкапа
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

/// Проверяет SHA-256 скачанного файла с эталоном из БД.
/// Выбрасывает ошибку, если хэши не совпадают (защита от подмены файла).
pub fn verify_file_hash(file_path: &str, expected_hash: &str) -> Result<(), String> {
    // Вычисляем реальный хэш файла, используя уже готовую функцию
    let actual_hash = calculate_file_hash(file_path)?;

    // Сравниваем (переводим ожидаемый в нижний регистр для надежности)
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
    
    // Превращаем массив байтов в hex-строку вручную
    let hex_string: String = result.iter().map(|b| format!("{:02x}", b)).collect();
    
    Ok(hex_string)
}