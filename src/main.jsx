import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Единая точка монтирования React-приложения.
// StrictMode помогает ловить побочные эффекты в dev-режиме.
ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
