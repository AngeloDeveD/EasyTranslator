import { useState, useEffect } from "react";

import Titlebar from "./components/titleBar/titleBar";
import { games } from "./data/games";
import Sidebar from "./components/Layout/Sidebar";
import MainContent from "./components/Layout/MainContent";
import { invoke } from "@tauri-apps/api/core";
//import TextEditor from "./components/textEditorComponent/textEditor";

import "./App.scss";

const FAKE_CATALOG_JSON = JSON.stringify([
  {
    id: "skyrim_se",
    name: "The Elder Scrolls V: Skyrim Special Edition",
    localizations: [
      {
        id: "skyrim_full_rus",
        version: "2.1.0",
        primary_url: "https://disk.yandex.ru/d/FAKE_SKYRIM_LINK",
        backup_url: null,
        archive_hash: "abc123hash",
        file_size_mb: 450,
        install_instructions: '[{"src": "strings/", "dest": "Data/strings/"}]',
        dll_whitelist: null
      }
    ]
  },
  {
    id: "persona4_golden",
    name: "Persona 4 Golden",
    localizations: [
      {
        id: "p4g_rus_text",
        version: "1.0.5",
        primary_url: "https://disk.yandex.ru/d/FAKE_PERSONA_LINK",
        backup_url: "https://my-server.com/backups/p4g_rus.zip",
        archive_hash: "def456hash",
        file_size_mb: 120,
        install_instructions: '[{"src": "data/text/", "dest": "data/text/"}]',
        dll_whitelist: '[{"name": "winmm.dll", "hash": "hash_of_winmm"}]' // Тест DLL
      }
    ]
  }
]);

export default function App() {

  //Тестовый список игр
  //const [activeGameId, setActiveGameId] = useState(games[0].id);
  const [games, setGames] = useState([]);

  const fetchGames = async () => {
    try {
      const result = await invoke("get_games");
      setGames(result);
    } catch (error) {
      console.error("Ошибка БД:", error);
    }
  };

  useEffect(() => {
    fetchGames();
  }, []);

  const handleTestSync = async () => {
    try {
      console.log("Отправляем фейковый каталог в Rust...");
      await invoke("sync_catalog", { jsonString: FAKE_CATALOG_JSON });
      console.log("Каталог успешно синхронизирован!");

      // После добавления данных, обновляем UI
      await fetchGames();
    } catch (error) {
      console.error("Ошибка синхронизации:", error);
    }
  };

  return (
    <div className="app">
      {/* 1. Слой тайтлбара */}
      <Titlebar />

      {/* 2. Слой основного контента */}
      <div className="app-body">
        <Sidebar games={games} />
        <MainContent games={games} />
        {/* ВРЕМЕННАЯ КНОПКА ТЕСТА (потом удалим) */}
        <button 
          onClick={handleTestSync} 
          style={{
            position: "fixed", bottom: "20px", right: "20px", zIndex: 9999,
            padding: "10px 15px", background: "var(--accent)", color: "#000",
            border: "none", borderRadius: "6px", fontWeight: "bold", cursor: "pointer"
          }}
        >
          🧪 Загрузить тестовый каталог
        </button>
      </div>
    </div>
  );

}
