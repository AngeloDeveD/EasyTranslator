import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  //const [name, setName] = useState("");

  const [resText, setResText] = useState("");

  function sendName(name){
    if (!name) return;

    try{
      setResText(invoke('greet', {name: name}));
    }
    catch(e){
      console.error("Ошибка от Rust:", e);
    }
  }

  return (
    <>
      <div>
        <h1>Привет, как тебя зовут</h1>
        <input type="text" onChange={e => sendName(e.target.value)} placeholder="Введите ваше имя"></input>
      </div>
      {resText && 
      <>
        <h2>{resText}</h2>
      </>}
    </>
  );

}

export default App;
