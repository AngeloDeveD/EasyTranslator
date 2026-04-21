export default function MainContent({ games, onOpenGame, onOpenAddGame }) {
  return (
    <main className="main">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "20px" }}>
        <h2 className="section-title" style={{ margin: 0 }}>Мои игры</h2>
        <button className="btn accent" onClick={onOpenAddGame}>+ Добавить игру</button>
      </div>
      <input className="search" placeholder="Поиск игр..." />
      
      <div className="grid" style={{ marginTop: "20px" }}>
        {games.length === 0 ? (
          // Состояние пустой БД/каталога.
          <p style={{ color: "var(--text-secondary)", gridColumn: "1 / -1" }}>База данных пуста.</p>
        ) : (
          // Карточка игры открывает detail-view с локализациями.
          games.map((game) => (
            <div className="card" key={game.id} style={{ height: "auto", cursor: "pointer" }} onClick={() => onOpenGame(game)}>
              <div className="card-content" style={{ gap: "10px" }}>
                <h3>{game.name}</h3>
                <p className="mods" style={{ fontSize: "12px", color: "var(--text-secondary)" }}>{game.description || "Нет описания"}</p>
              </div>
            </div>
          ))
        )}
      </div>
    </main>
  );
}
