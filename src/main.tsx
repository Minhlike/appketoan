import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("AppKetoan root element was not available during bootstrap");
}

function errorDetail(error: unknown): string {
  return error instanceof Error ? error.stack || error.message : String(error);
}

function AppBootstrap() {
  useEffect(() => {
    if (window.__APPKETOAN_BOOT__) {
      window.__APPKETOAN_BOOT__.mounted = true;
    }
    document.title = "AppKetoan — Đối chiếu số liệu kế toán";
  }, []);
  return <App />;
}

const root = ReactDOM.createRoot(rootElement, {
  onUncaughtError: (error) => {
    window.__APPKETOAN_BOOT__?.reportBootFailure(errorDetail(error));
  },
  onCaughtError: (error) => {
    console.error("React boundary caught an error", error);
  },
  onRecoverableError: (error) => {
    console.error("React recovered from an error", error);
  },
});

root.render(
  <React.StrictMode>
    <AppBootstrap />
  </React.StrictMode>,
);
