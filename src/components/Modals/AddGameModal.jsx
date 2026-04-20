import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AddGameModal({onClose, onGameAdded}){
    const [name, setName] = useState("");
    const [desc, setDesc] = useState("");

    const handleSubmit = async (e) => {
        e.preventDefault();
        if(!name.trim()) return;

        try{
            const newGameId = await invoke("add_local_game", {
                name: name,
                description: desc
            });

            onGameAdded({
                id: newGameId,
                name: name,
                description: desc || null,
                image_url: null,
                install_path: null
            });

            onClose();
        } catch (error){
            alert("Ошибка добавления: " + error);
        }
    };

    return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 9999
    }}>
      <div style={{
        background: "var(--bg-surface)", padding: "24px", borderRadius: "var(--radius-md)",
        border: "1px solid var(--border)", width: "400px"
      }}>
        <h2 style={{ marginBottom: "20px" }}>Добавить свою игру</h2>
        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
          
          <input 
            type="text" 
            placeholder="Название игры (например: Persona 5)" 
            value={name} 
            onChange={e => setName(e.target.value)}
            className="search" // Используем стили инпута поиска
            required
          />
          
          <textarea 
            placeholder="Краткое описание (необязательно)" 
            value={desc} 
            onChange={e => setDesc(e.target.value)}
            className="search" 
            rows={3}
            style={{ resize: "vertical" }}
          />

          <div style={{ display: "flex", gap: "10px", justifyContent: "flex-end" }}>
            <button type="button" className="btn secondary" onClick={onClose}>Отмена</button>
            <button type="submit" className="btn accent">Добавить</button>
          </div>
        </form>
      </div>
    </div>
  );
}