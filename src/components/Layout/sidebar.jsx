export default function Sidebar({ games, selectedGameId, onSelectGame, onOpenSettings }) {
  return (
    <aside className="sidebar">
      <h2>MOD LAUNCHER</h2>
      <div className="menu">
        {/* Список игр в левом меню. Активная игра подсвечивается по selectedGameId. */}
        {games.map((g) => (
          <div 
            key={g.id} 
            className={`menu-item ${g.id === selectedGameId ? "active" : ""}`}
            onClick={() => onSelectGame(g)}
          >
            {g.image_url ? (
              <img src={g.image_url} alt="" style={{ width: "20px", height: "20px", borderRadius: "4px", objectFit: "cover", marginRight: "10px" }} />
            ) : (
              <span style={{ marginRight: "10px" }}>🎮</span>
            )}
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{g.name}</span>
          </div>
        ))}
        {/* Настройки всегда доступны внизу бокового меню. */}
        <button className="menu-item" onClick={onOpenSettings} style={{ marginTop: "auto", color: "var(--text-secondary)", border: "none", background: "transparent" }}>
          ⚙️ Настройки
        </button>
      </div>
    </aside>
  );
}
