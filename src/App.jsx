import { useState } from "react";

import Titlebar from "./components/titleBar/titleBar";
import { games } from "./data/games";
import Sidebar from "./components/Layout/Sidebar";
import MainContent from "./components/Layout/MainContent";
//import TextEditor from "./components/textEditorComponent/textEditor";

import "./App.scss";

export default function App() {

  //Тестовый список игр
  const [activeGameId, setActiveGameId] = useState(games[0].id);

  return (
    <div className="app">
      {/* 1. Слой тайтлбара */}
      <Titlebar />

      {/* 2. Слой основного контента */}
      <div className="app-body">
        <Sidebar 
          games={games} 
          activeGameId={activeGameId} 
          onSelectGame={setActiveGameId} 
        />
        <MainContent games={games} />
      </div>
    </div>
  );

}
