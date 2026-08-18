# Current Task: Prompt 0 — Bootstrap Foundation

## Task Description
Prepare repository foundation, local development environment, directory architecture, Git repository, long-term AI memory system, security baseline, and build/test baseline for the offline accounting reconciliation desktop application.

## Scope
- [x] Check environment (Windows, Node.js, npm, Rust, Cargo, WebView2, C/C++ toolchain).
- [x] Configure toolchain (Rust stable GNU + MinGW w64devkit on Windows).
- [x] Initialize clean repository structure (`docs/`, `fixtures/`, `scripts/`, `tests/`, `src/`, `src-tauri/`, `releases/`, `.agent/`).
- [x] Configure security baseline in `.gitignore` (strictly protect real financial files, credentials, artifacts).
- [x] Establish AI memory system (`PROJECT_STATE.md`, `CURRENT_TASK.md`, `DECISIONS.md`, `HANDOFF.md`, `LESSONS.md`, `TEST_STATUS.md`, `AGENTS.md`).
- [x] Scaffold Tauri 2 + React + TypeScript + Vite minimal bootstrap skeleton displaying "Accounting Reconciliation — Bootstrap Ready".
- [x] Establish test baseline (Vitest frontend tests, Rust `cargo check` and `cargo test`).
- [x] Verify frontend build (`npm run build`) and Rust build (`cargo check`, `cargo test`).
- [x] Create clean initial git commit.

## Forbidden Actions (Strictly Enforced)
- Do NOT implement reconciliation algorithm engine in this session.
- Do NOT guess accounting rules not yet defined.
- Do NOT build complex upload UI or full dashboards.
- Do NOT introduce cloud services, database servers, telemetry, or external AI API calls.
- Do NOT commit real Excel files or customer accounting data.
- Do NOT start Prompt 1 prematurely.

## Task Status
- **Status**: COMPLETED & READY FOR PROMPT 1
