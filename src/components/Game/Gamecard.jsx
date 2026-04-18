import Status from "./Status";

export default function GameCard({ game }) {
  // Логика состояний для кнопок
  const getActions = (status) => {
    switch (status) {
      case "update": 
        return { primary: "Update", primaryClass: "btn accent" };
      case "error": 
        return { primary: "Fix Conflicts", primaryClass: "btn error" };
      case "success": 
      default: 
        return { primary: "▶ Play", primaryClass: "btn primary" };
    }
  };

  const { primary, primaryClass } = getActions(game.status);

  return (
    <div className="card">
      <div className="card-content">
        <h3>{game.name}</h3>
        <p className="mods">{game.mods} mods installed</p>
        
        <Status status={game.status} />

        <div className="actions">
          <button className="btn secondary">Manage</button>
          <button className={primaryClass}>{primary}</button>
        </div>
      </div>
    </div>
  );
}