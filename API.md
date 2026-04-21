# Smart Translator API

Документ описывает реальный контракт, который текущий клиент ожидает от API.
Источник: `src/App.jsx`, `src-tauri/src/db.rs`, `src-tauri/src/downloader.rs`.

## 1. Базовая идея

Клиент работает только с вашим API:

1. Получает каталог игр/локализаций.
2. Скачивает архив локализации для установки.
3. Отправляет предложение новой локализации (официальная ссылка + метаданные + optional картинка).

Дополнительно клиент поддерживает обновления: если локализация уже установлена и в каталоге появилась новая `version`, в UI появляется кнопка `Обновить`.

## 2. Конфигурация

Базовый URL API приходит из `VITE_CATALOG_API_BASE_URL`.

Пример:

```env
VITE_CATALOG_API_BASE_URL=https://api.example.com
```

Все `primary_url` в каталоге должны принадлежать этому же API-домену (и порту, и схеме). Иначе клиент отклонит запись.

## 3. Жизненный цикл в приложении

### 3.1 Старт приложения

1. React вызывает `sync_catalog_from_api`.
2. Backend делает `GET /api/v1/catalog`.
3. Ответ парсится и upsert-ится в SQLite.
4. UI читает карточки из локальной БД.

Если API недоступно, UI продолжает работать на старом локальном снимке.

### 3.2 Установка/обновление локализации

1. Backend берет из БД `primary_url`, `archive_hash`, `version`.
2. Проверяет, что URL принадлежат вашему API.
3. Загружает архив в `%AppData%/library/{localization_id}.zip`.
4. Проверяет SHA-256.
5. Делает backup оригиналов и распаковывает.
6. Пишет в `install_states.installed_version = текущая version`.

Если в следующей синхронизации версия в каталоге изменилась, то `installed_version != localizations.version`, и UI показывает `Обновить`.

### 3.3 Отправка новой локализации (предложение)

Пользователь в приложении вводит:

- имя/версию/язык/автора;
- официальную ссылку команды переводчиков (`source_url`);
- optional картинку (`png/jpg/jpeg/webp`);
- `install_instructions`.

Клиент отправляет данные на API и ожидает в ответ готовую запись локализации с `primary_url`, `archive_hash`, `file_size_mb`.

## 4. Endpoint: каталог

## `GET /api/v1/catalog`

### Request

- Method: `GET`
- Headers: `Accept: application/json`
- Body: отсутствует

### Response

Клиент принимает 2 формата:

1. Прямой массив игр.
2. Объект `{ "games": [...] }`.

### Поля игры

- `id` (string, required)
- `name` (string, required)
- `description` (string|null)
- `image_url` (string|null)
- `localizations` (array, required)

### Поля локализации

- `id` (string, required)
- `name` (string, required)
- `version` (string, required)
- `author` (string|null)
- `source_url` (string|null) — официальная ссылка проекта
- `image_url` (string|null) — изображение локализации
- `primary_url` (string, required)
- `archive_hash` (string, required, SHA-256 hex)
- `file_size_mb` (integer, required, >= 0)
- `install_instructions` (string, required; JSON-строка)
- `dll_whitelist` (string|null)

### Важные правила

1. `primary_url` должен принадлежать тому же API base URL.
2. При невалидном JSON/схеме клиент отклонит синхронизацию.
3. `install_path` пользователя при sync не перезаписывается.

## 5. Endpoint: скачивание архивов

Клиент скачивает по URL из каталога:

- `GET {primary_url}`

### Требования к ответу

- HTTP `2xx`
- бинарный zip в body
- `Content-Length` желательно (для прогресса)

### Проверка после скачивания

- SHA-256 zip должен совпасть с `archive_hash`.
- Иначе установка/обновление останавливается.

## 6. Endpoint: отправка предложения локализации

## `POST /api/v1/localizations/proposals`

Назначение: пользователь отправляет метаданные локализации и optional картинку, а API возвращает объект локализации, который уже можно установить через каталог/прямо из ответа.

### Request

- Method: `POST`
- Content-Type: `multipart/form-data`

Поля формы:

- `game_id` (string, required)
- `name` (string, required)
- `version` (string, required)
- `language` (string, required)
- `author` (string, optional)
- `source_url` (string, required, http/https)
- `install_instructions` (string, required; JSON-строка)
- `image` (file, optional; `png|jpg|jpeg|webp`)

### Response (успех)

Ожидается JSON со структурой локализации:

```json
{
  "id": "p4g_ru_community",
  "name": "Русский перевод",
  "version": "1.3.0",
  "author": "Community Team",
  "source_url": "https://vk.com/translator_team",
  "image_url": "https://api.example.com/storage/localizations/p4g_ru_community.jpg",
  "primary_url": "https://api.example.com/api/v1/localizations/p4g_ru_community/download",
  "archive_hash": "2c9f6f9f1d8e1f4fd0e0a53f3a0b6b9f0e6d0986a9c4d4efb73fcb78f2c1a123",
  "file_size_mb": 420,
  "install_instructions": "[{\"src\":\"Data/\",\"dest\":\"Data/\"}]",
  "dll_whitelist": null
}
```

Если API возвращает неуспех/невалидный JSON, клиент показывает ошибку пользователю и не добавляет локализацию в БД.

## 7. install_instructions

`install_instructions` всегда передается как строка JSON.

Пример:

```json
[
  { "src": "Data/", "dest": "Data/" }
]
```

Смысл:

- `src`: префикс пути внутри zip.
- `dest`: путь назначения внутри папки игры.

`[]` допустимо и означает "распаковать архив как есть".

## 8. Обновления локализаций

Серверу достаточно менять `version` при выпуске новой сборки.

Логика клиента:

1. При установке сохраняет `installed_version`.
2. После sync сравнивает `installed_version` с текущей `version`.
3. При расхождении показывает кнопку `Обновить`.
4. `Обновить` запускает стандартный pipeline установки с новым архивом.

## 9. Минимум для запуска backend API

1. Реализовать `GET /api/v1/catalog`.
2. Реализовать `POST /api/v1/localizations/proposals`.
3. Отдавать zip-архивы по `primary_url`.
4. Гарантировать корректный `archive_hash` и `file_size_mb`.
5. Держать `primary_url` в пределах вашего API base URL.

## 10. JSON примеры (что отправляет и получает программа)

Ниже только JSON-примеры для удобства backend-разработки.

### 10.1 Синхронизация каталога

HTTP:

- Request: `GET /api/v1/catalog`
- Body в запросе: отсутствует

Пример успешного ответа (вариант с оберткой):

```json
{
  "games": [
    {
      "id": "persona_4_golden",
      "name": "Persona 4 Golden",
      "description": "JRPG от ATLUS",
      "image_url": "https://api.example.com/storage/games/p4g.jpg",
      "localizations": [
        {
          "id": "p4g_ru_main",
          "name": "Русский перевод",
          "version": "1.3.0",
          "author": "Team Name",
          "source_url": "https://vk.com/team_name",
          "image_url": "https://api.example.com/storage/localizations/p4g_ru_main.jpg",
          "primary_url": "https://api.example.com/api/v1/localizations/p4g_ru_main/download",
          "archive_hash": "2c9f6f9f1d8e1f4fd0e0a53f3a0b6b9f0e6d0986a9c4d4efb73fcb78f2c1a123",
          "file_size_mb": 420,
          "install_instructions": "[{\"src\":\"Data/\",\"dest\":\"Data/\"}]",
          "dll_whitelist": null
        }
      ]
    }
  ]
}
```

Пример ошибки:

```json
{
  "error": {
    "code": "CATALOG_UNAVAILABLE",
    "message": "Catalog service temporarily unavailable"
  }
}
```

### 10.2 Отправка предложения локализации

Реально клиент шлет `multipart/form-data`, но ниже JSON-эквивалент полей (для понимания структуры данных):

```json
{
  "game_id": "persona_4_golden",
  "name": "Русский перевод",
  "version": "1.0.0",
  "language": "Русский",
  "author": "Team Name",
  "source_url": "https://vk.com/team_name",
  "install_instructions": "[{\"src\":\"Data/\",\"dest\":\"Data/\"}]",
  "image": {
    "filename": "p4g_cover.png",
    "mime": "image/png"
  }
}
```

Пример успешного ответа API (именно этот JSON ожидает клиент):

```json
{
  "id": "p4g_ru_community",
  "name": "Русский перевод",
  "version": "1.0.0",
  "author": "Team Name",
  "source_url": "https://vk.com/team_name",
  "image_url": "https://api.example.com/storage/localizations/p4g_ru_community.png",
  "primary_url": "https://api.example.com/api/v1/localizations/p4g_ru_community/download",
  "archive_hash": "41f314f4f063fe5542c9aa0ccf95f6fbbdd6d06d6434d9e0b35e8a3194fe2d8a",
  "file_size_mb": 512,
  "install_instructions": "[{\"src\":\"Data/\",\"dest\":\"Data/\"}]",
  "dll_whitelist": null
}
```

Пример ошибки:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "source_url must be a valid http/https URL",
    "details": {
      "field": "source_url"
    }
  }
}
```

### 10.3 Скачивание архива локализации

HTTP:

- Request: `GET {primary_url}`
- Request JSON body: отсутствует
- Response JSON body: отсутствует (возвращается бинарный zip)

Рекомендуемый JSON-формат ошибок (если архив недоступен):

```json
{
  "error": {
    "code": "ARCHIVE_NOT_FOUND",
    "message": "Localization archive not found"
  }
}
```
