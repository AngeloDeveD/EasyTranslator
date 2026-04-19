import GameCard from "../Game/GameCard";;

export default function MainContent({ games, onOpenGame }) {


  return (
    <main className="main">
      <input className="search" placeholder="Поиск игр..." />
      <h2 className="section-title">Мои игры</h2>
      
      <div className="grid">
        {games.length === 0 ? (
          <p style={{ color: "var(--text-secondary)", gridColumn: "1 / -1" }}>
            База данных пуста. Добавьте игры для отслеживания локализаций.
          </p>
        ) : (
          games.map((game) => (
            <GameCard key={game.id} game={game} onOpenGame={onOpenGame}/>
          ))
        )}
      </div>
    </main>
  );
}