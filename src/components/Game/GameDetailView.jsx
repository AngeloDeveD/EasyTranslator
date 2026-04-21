import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import Status from "../Game/Status";

export default function GameDetailView({ game, localizations, onBack, onSetPath, onAutoDetectPath, onResetPath, onInstall, onRollback, onOpenAddLoc, onDisable, onDelete }) {
  const path = game.install_path;

  // Последний payload прогресса загрузки/установки, приходящий из backend.
  const [progress, setProgress] = useState({});
  // Глобальная блокировка действий, пока идет установка/включение.
  const [isInstalling, setIsInstalling] = useState(false);

  // Подписка на Tauri event `download-progress`.
  useEffect(() => {
    const unlisten = listen("download-progress", (event) => setProgress(event.payload));
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const handleInstallClick = async (locId) => {
    if (isInstalling) return;
    setIsInstalling(true);
    setProgress({});
    try { await onInstall(locId); } 
    catch (error) { setProgress({}); } 
    finally { setIsInstalling(false); }
  };

  // Кнопки действий вычисляются от статуса локализации.
  const getInstallButton = (loc) => {
    if (loc.status === 'error') {
      return <button className="btn error" onClick={() => handleInstallClick(loc.id)} disabled={isInstalling}>Ошибка (Повторить)</button>;
    }

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

    if (loc.is_managed) {
      return (
        <div style={{ display: "flex", gap: "10px", width: "100%", marginTop: "10px" }}>
          
          {/* "Включить" показываем только для выключенного managed-перевода. */}
          {loc.status === 'available' && (
            <button className="btn accent" style={{ flex: 1 }} onClick={() => handleInstallClick(loc.id)} disabled={isInstalling}>
              Включить
            </button>
          )}

          {/* "Выключить" показываем только если файлы перевода сейчас активны. */}
          {loc.status === 'installed' && (
            <button className="btn secondary" style={{ flex: 1 }} onClick={() => onDisable(loc.id)} disabled={isInstalling}>
              Выключить
            </button>
          )}

          {/* "Удалить" всегда доступна для managed-перевода. */}
          <button className="btn error" style={{ flex: 1 }} onClick={() => onDelete(loc.id)} disabled={isInstalling}>
            Удалить
          </button>
        </div>
      );
    }

    // Каталожные переводы без локального жизненного цикла.
    return <button className="btn accent" onClick={() => handleInstallClick(loc.id)} disabled={isInstalling}>Установить</button>;
  }

  return (
    <>
       <button className="btn secondary" onClick={onBack} style={{ marginBottom: "20px", width: "fit-content" }}>← Назад</button>
      
      <div className="game-detail-header">
        {game.image_url && <img src={game.image_url} alt={game.name} className="game-detail-img" />}
        <div className="game-detail-info">
          <h1>{game.name}</h1>
          <p className="game-detail-desc">{game.description || "Описание отсутствует."}</p>
          <button className="btn secondary" onClick={onOpenAddLoc} style={{ marginTop: "15px" }}>+ Предложить свой перевод</button>
        </div>
      </div>

      {/* Блок без заданного install_path: показываем способы найти папку игры. */}
      {!path && (
        <div className="card" style={{ height: "auto", padding: "30px", textAlign: "center" }}>
          <h3 style={{ marginBottom: "15px" }}>Расположение игры не найдено</h3>
          <p style={{ color: "var(--text-secondary)", marginBottom: "20px" }}>
            Укажите папку с установленной игрой, чтобы продолжить установку перевода.
          </p>
          <div style={{ display: "flex", gap: "10px", justifyContent: "center" }}>
            <button className="btn secondary" onClick={() => onAutoDetectPath(game.id)}>🔍 Автопоиск</button>
            <button className="btn accent" onClick={() => onSetPath(game.id)}>📁 Указать вручную</button>
          </div>
        </div>
      )}

      {/* Блок с валидным install_path: доступен список переводов и действия. */}
      {path && (
        <>
          <div style={{ display: "flex", alignItems: "center", gap: "15px", marginBottom: "20px" }}>
            <Status status="success" text={`Путь найден: ${path}`} />
            <button onClick={() => onResetPath(game.id)} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", textDecoration: "underline", fontSize: "12px", padding: 0 }}>Сбросить путь</button>
          </div>

          {/* Оптимизированный рендер для одиночной локализации. */}
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

          {/* Сетка выбора, если для игры доступно несколько локализаций. */}
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
