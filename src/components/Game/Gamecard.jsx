export default function GameCard({ game, onOpenGame }) {
  return (
    // Клик по карточке открывает detail-view выбранной игры.
    <div className="card" onClick={() => onOpenGame(game)} style={{ cursor: "pointer" }}>
      <div className="card-content">
        <h3>{game.name}</h3>
        <p className="mods" style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "8px" }}>
          {game.description ? `${game.description.substring(0, 60)}...` : "Нет описания"}
        </p>
      </div>
    </div>
  );
}
