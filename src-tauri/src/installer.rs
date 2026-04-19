use std::fs::{self, File};
use std::path::PathBuf;
use zip::read::ZipArchive;

// Вспомогательная структура для парсинга JSON
#[derive(serde::Deserialize)]
struct MappingInstruction {
    src: String,
    dest: String,
}

//Функция распаковки архива
pub fn extract_archive(
    archive_path: &str, //Путь к скаченному файлу
    target_dir: &str, //Путь к папке с игрой
    instructions_json: &str,
) -> Result<(), String> {
    //Парсинг инструкций, которые хранятся в БД
    let instructions: Vec<MappingInstruction> = 
        serde_json::from_str(instructions_json)
        .map_err(|e| format!("Ошибка парсинга инструкций распаковки: {}", e))?;

    //Открываем ZIP архив
    let file = File::open(archive_path).map_err(|e| format!("Не удалось открыть архив: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Архив повреждён: {}", e))?;

    //Проходимся по всем файлам внутри архива
    for i in 0..archive.len(){
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

        //Смотрим файлы (скип папок)
        if file.is_file(){
            let file_name = file.name().to_string();

            println!("[INSTALLER] Найден файл в архиве: {}", file_name);

            // Ищем, подпадает ли файл под какую-то инструкцию
            for instr in &instructions {
                
                println!("[INSTALLER] Сверяю с правилом src='{}'", instr.src);

                if file_name.starts_with(&instr.src) {
                    // Отрезаем начало пути (src) и приклеиваем нужное (dest)
                    // Пример: "data/text.txt".starts_with("data/") -> отрезаем -> "text.txt" -> склеиваем с "Data/" -> "Data/text.txt"
                    let relative_path = file_name.strip_prefix(&instr.src).unwrap_or(&file_name);
                    let final_path = PathBuf::from(target_dir).join(&instr.dest).join(relative_path);

                    println!("[INSTALLER] СОВПАДЕНИЕ! Копирую в: {:?}", final_path);

                    //Создание нужных папок, если отсутсвует
                    if let Some(parent) = final_path.parent(){
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
