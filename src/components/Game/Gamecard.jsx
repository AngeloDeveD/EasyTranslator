import { invoke } from "@tauri-apps/api/core";

import Status from "./Status";

export default function GameCard({ game, onUpdate }) {
  // Вычисляем статус на основе данных из Rust
  const getGameStatus = () => {
    if (!game.install_path) {
      return { 
        uiStatus: "error", 
        statusText: "Путь не указан" 
      };
    }
    return { 
      uiStatus: "success", 
      statusText: "Игра найдена" 
    };
  };

  // Логика кнопок привязана к статусу игры
  const getActions = (uiStatus) => {
    if (uiStatus === "error") {
      return {
        primary: "Выбрать путь",
        primaryClass: "btn accent", // Кнопка attracting внимание (персовый)
        showSecondary: false
      };
    }

    // Если игра найдена
    return {
      primary: "Переводы",
      primaryClass: "btn primary",
      showSecondary: true,
      secondaryText: "Настроить"
    };
  };

  const { uiStatus, statusText } = getGameStatus();
  const { primary, primaryClass, showSecondary, secondaryText } = getActions(uiStatus);

  const handleSetPath = async () => {
    try{
      await invoke("set_game_path", {gameId: game.id});

      onUpdate();
    } catch (error){
      if(error !== "Выбор папки отменён"){
        console.error(error);
      }
    }
  };

  return (
    <div className="card">
      <div className="card-content">
        <h3>{game.name}</h3>
        
        <Status status={uiStatus} text={statusText} />

        <div className="actions">
          {showSecondary && (
            <button className="btn secondary">{secondaryText}</button>
          )}
          <button 
            className={primaryClass} 
            onClick={uiStatus === "error" ? handleSetPath : undefined}>
              {primary}
          </button>
        </div>
      </div>
    </div>
  );
}