import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi"; // Важно для Tauri v2

export default function SettingsView({ onClose }) {
  const [width, setWidth] = useState(1100);
  const [height, setHeight] = useState(700);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    appWindow.innerSize().then(size => { setWidth(size.width); setHeight(size.height); });
  }, []);

  const applySize = async () => {
    await appWindow.setSize(new LogicalSize(width, height));
  };

  return (
    <div style={{ maxWidth: "500px", margin: "0 auto", paddingTop: "40px" }}>
      <button className="btn secondary" onClick={onClose} style={{ marginBottom: "30px", width: "fit-content" }}>← Назад</button>
      <h2 style={{ marginBottom: "30px" }}>Настройки</h2>

      <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
        <div>
          <h3 style={{ fontSize: "16px", marginBottom: "15px", color: "var(--text-secondary)" }}>Размер окна</h3>
          <div style={{ display: "flex", gap: "10px", marginBottom: "15px" }}>
            <button className="btn secondary" onClick={() => { setWidth(1280); setHeight(720); }}>1280x720</button>
            <button className="btn secondary" onClick={() => { setWidth(1600); setHeight(900); }}>1600x900</button>
            <button className="btn secondary" onClick={() => { setWidth(1920); setHeight(1080); }}>Full HD</button>
          </div>
          <div style={{ display: "flex", gap: "10px", alignItems: "center" }}>
            <label style={{ fontSize: "14px", width: "40px" }}>Ш:</label>
            <input type="number" className="search" value={width} onChange={e => setWidth(Number(e.target.value))} style={{ width: "100px" }} />
            <label style={{ fontSize: "14px", width: "40px" }}>В:</label>
            <input type="number" className="search" value={height} onChange={e => setHeight(Number(e.target.value))} style={{ width: "100px" }} />
            <button className="btn accent" onClick={applySize}>Применить</button>
          </div>
        </div>
      </div>
    </div>
  );
}