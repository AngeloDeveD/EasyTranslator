import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function TextEditor() {

  // Вспомогательный dev-компонент для проверки Tauri-команд open/save_file.
  const [openButton, setOpenButton] = useState("Открыть файл");
  const [saveButton, setSaveButton] = useState("Сохранить файл");
  const [text, setText] = useState("");

  function openFile(){
    setOpenButton("Выбираем файл...");
    // Читает локальный текстовый файл через backend-команду.
    invoke("open_file")
      .then((content) => {
        setText(content);
        setOpenButton("Файл успешно открыт!!");
      })
      .catch(error => setText(error));
  }

  function saveFile(){
    setSaveButton("Сохраняем...");

    // Сохраняет текущее содержимое textarea в выбранный файл.
    invoke("save_file", {content: text})
      .then((successMsg) => {
        setSaveButton(successMsg);
      })
      .catch(error => setText(error));
  }

  return (
    <>
      <Titlebar />
      <h1>Простой блокнот</h1>
      <div>
        <button onClick={openFile}>{openButton}</button>
        <button onClick={saveFile}>{saveButton}</button>
      </div>

      <textarea placeholder="Тут отобразится текст" value={text} onChange={e =>setText(e.target.value)}></textarea>
    </>
  );

}
