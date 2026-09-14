import React from "react";
import ReactDOM from "react-dom/client";
import NewApp from "./newui/NewApp";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <NewApp />
  </React.StrictMode>,
);
