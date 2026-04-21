import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import Titlebar from "./components/titleBar/titleBar";
import Sidebar from "./components/Layout/Sidebar";
import MainContent from "./components/Layout/MainContent";
import GameDetailView from "./components/Game/GameDetailView";
import AddGameView from "./components/AddGameView";
import AddLocalizationView from "./components/AddLocalizationView";
import SettingsView from "./components/SettingsView";

import "./App.scss";

const API_BASE_URL = (import.meta.env.VITE_CATALOG_API_BASE_URL || "").trim();

export default function App() {
  const [games, setGames] = useState([]);
  const [selectedGame, setSelectedGame] = useState(null);
  const [localizations, setLocalizations] = useState([]);
  
  // UI-флаги: определяют, какой экран показывать в main-области.
  const [showSettings, setShowSettings] = useState(false);
  const [showAddGame, setShowAddGame] = useState(false);
  const [showAddLoc, setShowAddLoc] = useState(false);

  const refreshGamesFromDb = async () => {
    setGames(await invoke("get_games"));
  };

  const syncCatalogFromApi = async () => {
    if (!API_BASE_URL) {
      await refreshGamesFromDb();
      return;
    }

    try {
      await invoke("sync_catalog_from_api", { apiBaseUrl: API_BASE_URL });
    } catch (error) {
      // Не блокируем UI: при сетевой ошибке просто показываем последний локальный снимок БД.
      console.warn("Не удалось синхронизировать каталог с API:", error);
    }

    await refreshGamesFromDb();
  };

  useEffect(() => { syncCatalogFromApi(); }, []);

  const openGame = async (gameObj) => {
    // Экраны detail/settings взаимоисключающие.
    setShowSettings(false);
    setSelectedGame(gameObj);
    try {
      const result = await invoke("get_localizations", { gameId: gameObj.id });
      setLocalizations(result);
    } catch (error) { console.error("Ошибка получения локалей:", error); }
  };

  const goBack = () => {
    // Возврат к списку игр и чтение актуального состояния из БД.
    setSelectedGame(null);
    setLocalizations([]);
    refreshGamesFromDb();
  };

  // Включение перевода: скачать/взять из library, затем распаковать в игру.
  const handleInstall = async (locId) => {
    if (!API_BASE_URL) {
      alert("Не задан VITE_CATALOG_API_BASE_URL. Установка разрешена только через ваш API.");
      return;
    }

    try {
      await invoke("install_localization", { localizationId: locId, apiBaseUrl: API_BASE_URL });
      if (selectedGame) setLocalizations(await invoke("get_localizations", { gameId: selectedGame.id }));
    } catch (error) { alert("Ошибка включения: " + error); }
  };

  // Выключение перевода: откат файлов из backup.
  const handleDisable = async (locId) => {
    try {
      await invoke("disable_localization", { localizationId: locId });
      if (selectedGame) setLocalizations(await invoke("get_localizations", { gameId: selectedGame.id }));
    } catch (error) { alert("Ошибка выключения: " + error); }
  };

  // Полное удаление: откат + удаление backup + удаление архива из library.
  const handleDelete = async (locId) => {
    if (!confirm("Вы уверены? Это полностью удалит перевод из программы.")) return;
    try {
      await invoke("delete_localization", { localizationId: locId });
      if (selectedGame) setLocalizations(await invoke("get_localizations", { gameId: selectedGame.id }));
    } catch (error) { alert("Ошибка удаления: " + error); }
  };

  const handleSetPath = async (gameId) => {
    try {
      const newPath = await invoke("set_game_path", { gameId });
      setSelectedGame(prev => prev ? { ...prev, install_path: newPath } : null);
      refreshGamesFromDb();
    } catch (error) { if (error !== "Выбор папки отменен") alert("Ошибка: " + error); }
  };

  const handleAutoDetectPath = async (gameId) => {
    try {
      const newPath = await invoke("auto_detect_game_path", { gameId });
      setSelectedGame(prev => (prev ? { ...prev, install_path: newPath } : null));
      refreshGamesFromDb();
    } catch (error) { alert("Автопоиск не сработал: " + error); }
  };

  const handleResetPath = async (gameId) => {
    try {
      await invoke("reset_game_path", { gameId });
      setSelectedGame(prev => prev ? { ...prev, install_path: null } : null);
      refreshGamesFromDb();
    } catch (error) { console.error(error); }
  };

  const handleLocalGameAdded = (newGame) => { setGames(prev => [...prev, newGame]); setShowAddGame(false); };
  const handleLocalLocAdded = async () => {
    if (selectedGame) setLocalizations(await invoke("get_localizations", { gameId: selectedGame.id }));
    setShowAddLoc(false);
  };
  const handleOpenSettings = () => setShowSettings(true);

  return (
    <div className="app">
      <Titlebar />
      <div className="app-body">
        <Sidebar games={games} selectedGameId={selectedGame?.id} onSelectGame={openGame} onOpenSettings={handleOpenSettings} />
        
        <main className="main">
          {/* Рендер main-экрана строго взаимоисключающий: */}
          {/* Settings -> Games list/AddGame -> Game detail/AddLocalization */}
          {showSettings ? (
            <SettingsView onClose={() => setShowSettings(false)} />
          ) : !selectedGame ? (
            showAddGame ? (
              <AddGameView onClose={() => setShowAddGame(false)} onGameAdded={handleLocalGameAdded} />
            ) : (
              <MainContent games={games} onOpenGame={openGame} onOpenAddGame={() => setShowAddGame(true)} onOpenSettings={handleOpenSettings} />
            )
          ) : showAddLoc ? (
            <AddLocalizationView
              gameId={selectedGame.id}
              apiBaseUrl={API_BASE_URL}
              onClose={() => setShowAddLoc(false)}
              onLocAdded={handleLocalLocAdded}
            />
          ) : (
            <GameDetailView 
              game={selectedGame} localizations={localizations} onBack={goBack}
              onSetPath={handleSetPath} onAutoDetectPath={handleAutoDetectPath} onResetPath={handleResetPath}
              onInstall={handleInstall} onDisable={handleDisable} onDelete={handleDelete}
              onOpenAddLoc={() => setShowAddLoc(true)}
            />
          )}
        </main>
      </div>
    </div>
  );
}
