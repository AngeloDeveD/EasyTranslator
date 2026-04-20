//Установка фалйа с поддержкой фоллбэка (рез. ссылка)
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
    file_name: &str, //Название архива
) -> Result<PathBuf, String> {
    match download_file(app.clone(), primary_url, file_name).await {
        Ok(path) => return Ok(path),
        Err(primary_err) => {
            println!("[DOWNLOAD] Ошибка primary URL: {}. Пробуем backup...", primary_err);

            //Есть оригинальная ссылка упала, то скачивает с backup

            if let Some(backup) = backup_url {
                return download_file(app, backup, file_name).await;
            }

            //Если нет бэкапа, то возвращает ошибку
            Err(format!("Не удалось скачать файл. Основная и резервная ссылка недоступна. Ошибка: {}", primary_err))
        }
    }
}

/// Внутренняя функция скачивания одного файла по кусочкам
async fn download_file(
    app: tauri::AppHandle,
    url: &str,
    file_name: &str,
) -> Result<PathBuf, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30)) // Снизили таймаут с 300 сек до 30
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {}", e))?;

    let response = client.get(url).send().await
        .map_err(|e| format!("Ошибка сети: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Сервер вернул ошибку: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    
    // ПИШЕМ НАПРЯМУЮ В %TEMP%
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