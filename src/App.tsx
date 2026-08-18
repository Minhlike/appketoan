import "./App.css";

function App() {
  return (
    <main className="app-container">
      <div className="card">
        <div className="status-badge">System Status: Online</div>
        <h1 className="title">Accounting Reconciliation</h1>
        <p className="subtitle">Bootstrap Ready</p>
        <div className="info-panel">
          <div className="info-row">
            <span className="label">Mode:</span>
            <span className="value">Local Desktop (Offline-First)</span>
          </div>
          <div className="info-row">
            <span className="label">Multi-Source Engine:</span>
            <span className="value">Ready for Phase 1 Data Contract</span>
          </div>
          <div className="info-row">
            <span className="label">Telemetry / Cloud:</span>
            <span className="value">Disabled (Zero Exfiltration)</span>
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
