export default function Sidebar({ games, selectedGameId, onSelectGame }) {
  return (
    <aside className="sidebar">
      <h2>MOD LAUNCHER</h2>
      <div className="menu">
        {games.map((g) => (
          <div 
            key={g.id} 
            className={`menu-item ${g.id === selectedGameId ? "active" : ""}`}
            onClick={() => onSelectGame(g)} // Передаем весь объект g
          >
            {g.image_url ? (
              <img 
                src={g.image_url} 
                alt="" 
                style={{ 
                  width: "20px", 
                  height: "20px", 
                  borderRadius: "4px", 
                  objectFit: "cover",
                  marginRight: "10px"
                }} 
              />
            ) : (
              <span style={{ marginRight: "10px" }}>🎮</span>
            )}
            <span style={{ 
                overflow: "hidden", 
                textOverflow: "ellipsis", 
                whiteSpace: "nowrap",
                flex: 1
            }}>
              {g.name}
            </span>
          </div>
        ))}
      </div>
    </aside>
  );
}