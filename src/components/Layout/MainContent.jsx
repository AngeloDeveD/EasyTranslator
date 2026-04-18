import GameCard from "../Game/GameCard";

export default function MainContent({ games }) {
  return (
    <main className="main">
      <input className="search" placeholder="Search mods or games..." />
      <h2 className="section-title">Installed</h2>
      
      <div className="grid">
        {games.map((game) => (
          <GameCard key={game.id} game={game} />
        ))}
      </div>
    </main>
  );
}