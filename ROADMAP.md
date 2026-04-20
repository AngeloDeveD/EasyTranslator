[![English](https://img.shields.io/badge/Readme-English-blue.svg)](README.md)
[![Русский](https://img.shields.io/badge/Readme-Руссикй-green.svg)](README_rus.md)
[![Реализация](https://img.shields.io/badge/Реализовано-70%25-red)](README_rus.md)


# 🗺 Roadmap

# Phase 1: Foundation & UI Shell (Каркас)
- [X] Базовый каркас приложения (Tauri v2 + React)
- [X] Настройка ограничений окна (min-width/min-height, запрет на излом интерфейса)
- [X] Уменьшение размера билда (оптимизация зависимостей)
- [X] Кастомный Titlebar (кнопки свернуть/закрыть,原生 drag region)
- [X] Design System (SCSS, :root переменные, холодная тема, State-driven цвета)
- [X] Инициализация SQLite (настройка WAL режима, Foreign Keys)
- [X] Архитектура БД (Таблицы: games, localizations, install_states)
- [X] Интеграция Mutex<Connection> в Tauri State (потокобезопасный доступ к БД из UI)
- [X] Компонентная архитектура (Wizard-флоу: Список игр -> Детали игры -> Выбор перевода)
- [X] Взаимодействие с ОС (Интеграция rfd для выбора папки с игрой)
- [ ] Создание полного дизайна в Figma

# Phase 2: Core Engine & Security (Ядро)
- [X] Базовый движок распаковки (Интеграция zip кейта, распаковка по правилам)
- [X] Система маппинга путей (install_instructions JSON: срезание лишних папок из архивов)
- [X] Неявный Safe Extractor (файлы, не описанные в JSON-манифесте, игнорируются)
- [X] Система бэкапов (create_backup -> AppData/backups)
- [X] Rollback (restart_backup -> Восстановление оригиналов)
- [X] Smart File Validator (Блокировка .exe/.bat, белый список .dll)
- [X] Hash Verification (SHA-256 проверка ДО распаковки)
- [X] Защита от Zip-Slip (проверка выхода за пределы папки игры)

# Phase 3: Network & Catalog (Сеть)
- [X] Формат Манифеста (Строгие serde структуры для JSON каталога)
- [X] Синхронизация БД при старте (ON CONFLICT DO UPDATE)
- [X] HTTP-клиент (reqwest, асинхронный стриминг)
- [X] Failover Download Manager (primary_url -> backup_url)
- [X] State-driven UI загрузок (app.emit -> listen -> Прогресс-бар)
- [X] Local Library Architecture (Скачивание в AppData/library, повторная установка без интернета, включение/выключение/удаление)

# Phase 4: UX & Error Handling (Опыт использования)
- [ ] Настройки окна (Динамическое изменение разрешения из UI)
- [ ] Локализация системных ошибок (Перевод Rust OS ошибок в человекочитаемый вид)
- [ ] Проверка процессов (Блокировка кнопок, если .exe игры запущен)
- [ ] Обработка нехватки места на диске (Предупреждение перед скачиванием/распаковкой)
- [ ] Кнопка "Ссылка не работает" (Жалоба на нерабочий primary_url)
- [ ] Автопоиск игр (Парсинг Steam/Epic реестров)

# Phase 5: Production & Distribution (Релиз)
- [ ] Логирование
- [ ] Tauri Updater (Автообновление лаунчера)
- [ ] Сборка инсталлятора (NSIS/WiX)

# ⚠️ Известные ограничения
Форматы: Только .zip (нет 7z/rar).
FTP: Не поддерживается (только HTTP/HTTPS).

## ... ну и самое главное ...
- [ ] Принятие того, через что придётся пройти...