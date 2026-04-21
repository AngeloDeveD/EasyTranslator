import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AddGameView({ onClose, onGameAdded }) {
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [imageUrl, setImageUrl] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!name.trim()) return;
    setIsSaving(true);

    try {
      const newGameId = await invoke("add_local_game", { 
        name: name, 
        description: desc, 
        // Бэкенд ожидает `null`, если обложка не указана.
        imageUrl: imageUrl || null
      });
      
      onGameAdded({ 
        id: newGameId, 
        name, 
        description: desc || null, 
        image_url: imageUrl || null, 
        install_path: null 
      });
      onClose();
    } catch (error) {
      alert("Ошибка: " + error);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div style={{ maxWidth: "600px", paddingTop: "40px" }}>
      <button 
        className="btn secondary" 
        onClick={onClose} 
        style={{ marginBottom: "30px", width: "fit-content" }}
      >
        ← Назад к списку
      </button>

      <h2 style={{ marginBottom: "30px" }}>Добавить игру</h2>

      <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
      <div>
        <label style={styles.label}>Название игры *</label>
        <input className="search" value={name} onChange={e => setName(e.target.value)} required disabled={isSaving} />
      </div>
      {/* URL обложки хранится как image_url и используется в sidebar/detail-view. */}
      <div>
        <label style={styles.label}>Ссылка на обложку (URL)</label>
        <input className="search" placeholder="https://images.igdb.com/..." value={imageUrl} onChange={e => setImageUrl(e.target.value)} disabled={isSaving} />
      </div>

      <div>
        <label style={styles.label}>Описание</label>
        <textarea className="search" value={desc} onChange={e => setDesc(e.target.value)} rows={3} disabled={isSaving} />
      </div>

      <button type="submit" className="btn accent" style={{ width: "fit-content" }} disabled={isSaving}>
        {isSaving ? "Сохранение..." : "Добавить игру"}
      </button>
    </form>
    </div>
  );
}

const styles = {
  label: { display: "block", marginBottom: "8px", color: "var(--text-secondary)", fontSize: "13px" }
};
