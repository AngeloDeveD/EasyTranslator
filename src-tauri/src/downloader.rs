// Слой скачивания архивов с поддержкой fallback-ссылки.
// Отдельный модуль нужен, чтобы держать сетевую логику вне `db.rs`.
use reqwest;
use std::path::PathBuf;
use futures_util::StreamExt;
use tauri::Emitter;
use std::io::Write;
use std::env;

pub async fn download_with_fallback(
    app: tauri::AppHandle,
    primary_url: &str,
    backup_url: Option<&str>,
    file_name: &str, // Имя временного файла в системной temp-папке.
) -> Result<PathBuf, String> {
    match download_file(app.clone(), primary_url, file_name).await {
        Ok(path) => return Ok(path),
        Err(primary_err) => {
            println!("[DOWNLOAD] Ошибка primary URL: {}. Пробуем backup...", primary_err);

            // При падении primary_url пробуем backup_url (если он задан).
            if let Some(backup) = backup_url {
                return download_file(app, backup, file_name).await;
            }

            // Если резервной ссылки нет, пробрасываем исходную ошибку выше.
            Err(format!("Не удалось скачать файл. Основная и резервная ссылка недоступна. Ошибка: {}", primary_err))
        }
    }
}

/// Скачивает файл потоково и пишет его напрямую на диск.
/// Во время скачивания публикует прогресс для UI через `download-progress`.
async fn download_file(
    app: tauri::AppHandle,
    url: &str,
    file_name: &str,
) -> Result<PathBuf, String> {
    let client = reqwest::Client::builder()
        // Короткий timeout нужен, чтобы быстрее переключаться на fallback-ссылку.
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {}", e))?;

    let response = client.get(url).send().await
        .map_err(|e| format!("Ошибка сети: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Сервер вернул ошибку: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    
    // Сначала сохраняем файл во временную папку. Перемещение в library выполняется выше по стеку.
    let temp_dir = env::temp_dir();
    let final_path = temp_dir.join(file_name);
    
    let mut out_file = std::fs::File::create(&final_path)
        .map_err(|e| format!("Нет прав на создание временного файла: {}", e))?;
    
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Ошибка чтения потока: {}", e))?;
        
        out_file.write_all(&chunk).map_err(|e| format!("Ошибка записи на диск: {}", e))?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            // Защита от деления на ноль и лишнего UI-шума при неизвестной длине контента.
            let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
            let _ = app.emit("download-progress", serde_json::json!({
                "percent": percent,
                "downloaded_mb": (downloaded as f64 / 1_048_576.0).round() as u32,
                "total_mb": (total_size as f64 / 1_048_576.0).round() as u32
            }));
        }
    }

    println!("[DOWNLOAD] Файл успешно скачан: {:?}", final_path);
    Ok(final_path)
}
