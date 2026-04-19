import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Status from "../Game/Status";

export default function GameDetailView({ game, localizations, onBack, onSetPath, onResetPath, onInstall }) {
  const path = game.install_path;

  // Стейт для прогресса (привязан к ID локализации)
  const [progress, setProgress] = useState({});

  // Слушатель событий из Rust
  useEffect(() => {
    const unlisten = listen("download-progress", (event) => {
      // event.payload содержит { percent: 45, downloaded_mb: 10, total_mb: 100 }
      // Мы не знаем, для какого перевода качается файл, поэтому обновляем общий стейт
      // (Для простоты считаем, что качается только один файл одновременно)
      setProgress(event.payload);
    });

    return () => {
      unlisten.then(fn => fn()); // Отписываемся при размонтировании
    };
  }, []);

  const setPath = async () => {
    try {
      await invoke("set_game_path", { gameId: game.id });
      // Просто перезагружаем страницу detail view, обновляя стейт родителя
      onBack();
      // Временный костыль: после обновления сразу возвращаемся в игру.
      // Позже мы сделаем это элегантнее через возврат нового пути из Rust
      setTimeout(() => onBack(), 50); 
    } catch (e) { if (e !== "Выбор папки отменен") console.error(e); }
  };

  const handleAddTranslation = () => {
    alert("Здесь будет форма отправки архива модератору (пока заглушка)");
  };

  const getInstallButton = (loc) => {
    // Если статус downloading ИЛИ installing ИЛИ есть прогресс (процент > 0) -> показываем прогресс
    if ((loc.status === 'downloading' || loc.status === 'installing') && progress.percent > 0) {
      return (
        <div style={{ width: "100%", marginTop: "10px" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: "12px", marginBottom: "5px", color: "var(--text-secondary)" }}>
            <span>{loc.status === 'downloading' ? 'Скачивание...' : 'Распаковка...'}</span>
            <span>{progress.percent}% ({progress.downloaded_mb} / {progress.total_mb} MB)</span>
          </div>
          <div style={{ width: "100%", height: "4px", background: "var(--bg-elevated)", borderRadius: "2px" }}>
            <div style={{ width: `${progress.percent}%`, height: "100%", background: "var(--accent)", borderRadius: "2px", transition: "width 0.2s" }}></div>
          </div>
        </div>
      );
    }

    if (loc.status === 'installed') return <button className="btn secondary">Удалить перевод</button>;
    
    return <button className="btn accent" onClick={() => onInstall(loc.id)}>Установить</button>;
  };

  return (
    <>
      <button className="btn secondary" onClick={onBack} style={{ marginBottom: "20px", width: "fit-content" }}>
        ← Назад
      </button>
      
       {/* Шапка с обложкой и инфо */}
      <div className="game-detail-header">
        {game.image_url && (
          <img 
            src={game.image_url} 
            alt={game.name} 
            className="game-detail-img"
            // В Tauri нужно разрешить загрузку картинок с других сайтов в tauri.conf.json (allowlist)
          />
        )}
        <div className="game-detail-info">
          <h1>{game.name}</h1>
          <p className="game-detail-desc">{game.description || "Описание отсутствует."}</p>
          
          <button className="btn secondary" onClick={handleAddTranslation} style={{ marginTop: "15px" }}>
            ✉️ Предложить свой перевод
          </button>
        </div>
      </div>


       {/* БЛОК 1: ПУТЬ НЕ НАЙДЕН */}
      {!path && (
        <div className="card" style={{ height: "auto", padding: "30px", textAlign: "center" }}>
          <h3 style={{ marginBottom: "15px" }}>Расположение игры не найдено</h3>
          <p style={{ color: "var(--text-secondary)", marginBottom: "20px" }}>
            Укажите папку с установленной игрой, чтобы продолжить установку перевода.
          </p>
          <div style={{ display: "flex", gap: "10px", justifyContent: "center" }}>
            <button className="btn secondary" disabled>🔍 Автопоиск</button>
            {/* ПРОСТО ВЫЗЫВАЕМ onSetPath */}
            <button className="btn accent" onClick={() => onSetPath(game.id)}>📁 Указать вручную</button>
          </div>
        </div>
      )}

       {/* БЛОК 2: ПУТЬ НАЙДЕН */}
      {path && (
        <>
          {/* Обернули в flex, чтобы статус и кнопка были на одной строке */}
          <div style={{ display: "flex", alignItems: "center", gap: "15px", marginBottom: "20px" }}>
            <Status status="success" text={`Путь найден: ${path}`} />
            
            {/* КНОПКА СБРОСА */}
            <button 
              onClick={() => onResetPath(game.id)} 
              style={{
                background: "none", 
                border: "none", 
                color: "var(--text-secondary)", 
                cursor: "pointer", 
                textDecoration: "underline", 
                fontSize: "12px",
                padding: 0
              }}
            >
              Сбросить путь
            </button>
          </div>

          {/* Перевод ровно 1 */}
          {localizations.length === 1 && (
            <div className="card" style={{ height: "auto", marginTop: "20px" }}>
              <div className="card-content" style={{ gap: "10px" }}>
                <h3>{localizations[0].name}</h3>
                <p className="mods">
                  Версия: {localizations[0].version} • Размер: {localizations[0].file_size_mb} MB
                </p>
                {localizations[0].author && (
                  <p className="mods">
                    Автор: <a href={localizations[0].source_url} target="_blank" style={{ color: "var(--accent)", textDecoration: "none" }}>{localizations[0].author}</a>
                  </p>
                )}
                {getInstallButton(localizations[0])}
              </div>
            </div>
          )}

          {/* Переводов несколько (Выбор) */}
          {localizations.length > 1 && (
            <div className="grid" style={{ marginTop: "20px" }}>
              {localizations.map((loc) => (
                <div className="card" key={loc.id} style={{ height: "auto" }}>
                  <div className="card-content" style={{ gap: "10px" }}>
                    <h3>{loc.name}</h3>
                    <p className="mods">Версия: {loc.version}</p>
                    <p className="mods">Размер: {loc.file_size_mb} MB</p>
                    {loc.author && (
                      <p className="mods">
                        Автор: <a href={loc.source_url} target="_blank" style={{ color: "var(--accent)", textDecoration: "none" }}>{loc.author}</a>
                      </p>
                    )}
                    {getInstallButton(loc)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </>
  );
}