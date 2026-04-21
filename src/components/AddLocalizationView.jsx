import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function AddLocalizationView({ gameId, onClose, onLocAdded }) {
  const [form, setForm] = useState({
    name: "",
    version: "1.0",
    language: "Русский",
    author: "",
    filePath: "",       // Абсолютный путь к локальному zip-архиву.
    instructions: "[]"
  });
  const [isSaving, setIsSaving] = useState(false);

  const handlePickFile = async () => {
    const path = await invoke("pick_localization_file");
    if (path) {
      setForm({ ...form, filePath: path });
    }
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!form.name.trim() || !form.filePath) {
      alert("Укажите название и выберите файл архива!");
      return;
    }

    setIsSaving(true);
    try {
      await invoke("add_local_localization", {
        gameId: gameId,
        name: form.name,
        version: form.version,
        language: form.language,
        author: form.author,
        // Backend сам вычисляет hash/size и сохраняет архив как локальную локализацию.
        filePath: form.filePath,
        instructionsJson: form.instructions
      });

      onLocAdded();
      onClose();
    } catch (error) {
      alert("Ошибка сохранения: " + error);
    } finally {
      setIsSaving(false);
    }
  };

  // В UI показываем только имя файла, чтобы не растягивать строку абсолютным путем.
  const getFileName = (path) => {
    if (!path) return "";
    const parts = path.replace(/\\/g, "/").split("/");
    return parts[parts.length - 1];
  };

  return (
    <div style={{ maxWidth: "600px", margin: "0 auto", paddingTop: "40px" }}>
      <button className="btn secondary" onClick={onClose} style={{ marginBottom: "30px", width: "fit-content" }}>← Назад к игре</button>
      <h2 style={{ marginBottom: "30px" }}>Добавить перевод</h2>

      <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
        
        <div>
          <label style={styles.label}>Название перевода *</label>
          <input className="search" placeholder="Полный перевод текста" value={form.name} onChange={e => setForm({...form, name: e.target.value})} required disabled={isSaving} />
        </div>

        <div style={{ display: "flex", gap: "15px" }}>
          <div style={{ flex: 1 }}>
            <label style={styles.label}>Версия</label>
            <input className="search" value={form.version} onChange={e => setForm({...form, version: e.target.value})} disabled={isSaving} />
          </div>
          <div style={{ flex: 1 }}>
            <label style={styles.label}>Язык перевода</label>
            <select className="search" value={form.language} onChange={e => setForm({...form, language: e.target.value})} style={{ cursor: "pointer", appearance: "auto" }} disabled={isSaving}>
              <option value="Русский">Русский</option>
              <option value="Английский">Английский</option>
              <option value="Японский">Японский</option>
            </select>
          </div>
        </div>

        <div>
          <label style={styles.label}>Автор / Команда</label>
          <input className="search" placeholder="Название команды" value={form.author} onChange={e => setForm({...form, author: e.target.value})} disabled={isSaving} />
        </div>

        {/* Выбор локального архива перевода. */}
        <div>
          <label style={styles.label}>Архив с переводом (.zip) *</label>
          <div style={{ display: "flex", gap: "10px" }}>
            <input 
              className="search" 
              value={form.filePath ? getFileName(form.filePath) : ""} 
              readOnly 
              placeholder="Файл не выбран" 
              style={{ flex: 1, color: form.filePath ? "var(--text-primary)" : "var(--text-secondary)" }}
            />
            <button type="button" className="btn secondary" onClick={handlePickFile} disabled={isSaving}>
              Обзор...
            </button>
          </div>
        </div>

        {/* Продвинутый режим: ручные install_instructions для нестандартной структуры архива. */}
        <details style={{ cursor: "pointer" }}>
          <summary style={{ color: "var(--text-secondary)", fontSize: "13px", marginBottom: "10px" }}>
            ⚙️ Продвинутые настройки (JSON инструкции)
          </summary>
          <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "10px" }}>
            Оставьте пустым, чтобы просто распаковать архив в папку игры. Заполняйте ТОЛЬКО если архив содержит лишние папки, которые нужно "срезать" (например, если внутри архива есть папка `archive/`, а файлы нужно положить в корень).
          </p>
          <textarea 
            className="search" 
            placeholder='[{"src": "archive/", "dest": ""}]' 
            value={form.instructions} 
            onChange={e => setForm({...form, instructions: e.target.value})} 
            rows={3} 
            style={{ fontFamily: "monospace", fontSize: "12px" }} 
            disabled={isSaving} 
          />
        </details>

        <button type="submit" className="btn accent" style={{ width: "fit-content" }} disabled={isSaving}>
          {isSaving ? "Вычисление хэша и сохранение..." : "Добавить перевод"}
        </button>
      </form>
    </div>
  );
}

const styles = {
  label: { display: "block", marginBottom: "8px", color: "var(--text-secondary)", fontSize: "13px" }
};
