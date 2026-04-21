// legacy/mock данные для ранних UI-прототипов.
// Актуальный список игр сейчас приходит из SQLite через Tauri-команду `get_games`.
export const games = [
  { id: 1, name: "Skyrim", mods: 124, status: "update" },
  { id: 2, name: "Cyberpunk 2077", mods: 54, status: "success" },
  { id: 3, name: "Witcher 3", mods: 87, status: "error" },
];
