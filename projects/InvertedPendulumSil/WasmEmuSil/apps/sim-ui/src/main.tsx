import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@ipemu/ui-kit/theme.css";
import "./app.css";
import { App } from "./App";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
