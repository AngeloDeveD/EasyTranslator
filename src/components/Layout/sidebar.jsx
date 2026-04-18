export default function Sidebar({ games, activeGameId, onSelectGame }) {
  return (
    <aside className="sidebar">
      <h2>MOD LAUNCHER</h2>
      <div className="menu">
        {games.map((g) => (
          <div 
            key={g.id} 
            className={`menu-item ${g.id === activeGameId ? "active" : ""}`}
            onClick={() => onSelectGame(g.id)}
          >
            🎮 {g.name}
          </div>
        ))}
      </div>
    </aside>
  );
}