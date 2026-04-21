import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import Titlebar from "./components/titleBar/titleBar";
import Sidebar from "./components/Layout/Sidebar";
import MainContent from "./components/Layout/MainContent";
import GameDetailView from "./components/Game/GameDetailView";
import AddGameModal from "./components/Modals/AddGameModal";
import AddGameView from "./components/AddGameView";
import AddLocalizationView from "./components/AddLocalizationView";
import SettingsView from "./components/SettingsView";

import "./App.scss";

const FAKE_CATALOG_JSON = JSON.stringify([
  // Локальный seed-каталог для разработки:
  // используется, если БД пустая на первом запуске.
  {
    id: "game_1",
    name: "Игра 1",
    image_url: "https://images.igdb.com/igdb/image/upload/t_cover_big/co5vmg.webp",
    description: "Тестовая игра",
    localizations: [
      {
        id: "loc_1_text",
        name: "Текстовый перевод",
        version: "1.0",
        // Оба перевода пишут в одну зону, что полезно для теста конфликтов.
        primary_url: "https://raw.githubusercontent.com/torvalds/linux/master/README",
        archive_hash: "abc123",
        file_size_mb: 10,
        install_instructions: JSON.stringify([{ "src": "data/", "dest": "Data/" }])
      },
      {
        id: "loc_1_sound",
        name: "Перевод озвучки",
        version: "1.0",
        primary_url: "https://raw.githubusercontent.com/torvalds/linux/master/README",
        archive_hash: "def456",
        file_size_mb: 50,
        // Этот перевод тоже целится в `Data/` -> ожидаем конфликт при одновременной активации.
        install_instructions: JSON.stringify([{ "src": "data/sound.pak", "dest": "Data/" }])
      }
    ]
  }
]);

export default function App() {
  const [games, setGames] = useState([]);
  const [selectedGame, setSelectedGame] = useState(null);
  const [localizations, setLocalizations] = useState([]);
  
  // UI-флаги: определяют, какой экран показывать в main-области.
  const [showSettings, setShowSettings] = useState(false);
  const [showAddGame, setShowAddGame] = useState(false);
  const [showAddLoc, setShowAddLoc] = useState(false);

  const checkAndSync = async () => {
    // На пустой БД подгружаем тестовый каталог, чтобы интерфейс сразу был интерактивным.
    const currentGames = await invoke("get_games");
    if (currentGames.length === 0) {
      try { await invoke("sync_catalog", { jsonString: FAKE_CATALOG_JSON }); }
      catch (error) { console.error("Ошибка синхронизации:", error); }
    }
    setGames(await invoke("get_games"));
  };

  useEffect(() => { checkAndSync(); }, []);

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
    // Возврат к списку игр и синхронизация свежего состояния из БД.
    setSelectedGame(null);
    setLocalizations([]);
    checkAndSync();
  };

  // Включение перевода: скачать/взять из library, затем распаковать в игру.
  const handleInstall = async (locId) => {
    try {
      await invoke("install_localization", { localizationId: locId });
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
      checkAndSync();
    } catch (error) { if (error !== "Выбор папки отменен") alert("Ошибка: " + error); }
  };

  const handleAutoDetectPath = async (gameId) => {
    try {
      const newPath = await invoke("auto_detect_game_path", { gameId });
      setSelectedGame(prev => (prev ? { ...prev, install_path: newPath } : null));
      checkAndSync();
    } catch (error) { alert("Автопоиск не сработал: " + error); }
  };

  const handleResetPath = async (gameId) => {
    try {
      await invoke("reset_game_path", { gameId });
      setSelectedGame(prev => prev ? { ...prev, install_path: null } : null);
      checkAndSync();
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
            <AddLocalizationView gameId={selectedGame.id} onClose={() => setShowAddLoc(false)} onLocAdded={handleLocalLocAdded} />
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
