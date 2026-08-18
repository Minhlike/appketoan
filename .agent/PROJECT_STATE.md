# Project State

## Product Goal
A standalone, 100% offline desktop application for Windows to perform automated multi-source accounting data reconciliation across diverse Excel workbooks (electronic invoices, general ledgers, bank statements, multi-branch books, custom workbooks).

## Core Technical Stack
- **Desktop Framework**: Tauri v2
- **Backend / Core Engine**: Rust (stable-x86_64-pc-windows-gnu toolchain with GCC 16.2.0 / w64devkit)
- **Frontend**: React 19 + TypeScript + Vite
- **Styling**: Vanilla CSS (clean, professional, function-driven)
- **Testing**: Vitest + Testing Library (Frontend), `cargo test` (Rust Backend)
- **VCS**: Git

## High-Level Architecture
```text
UI (React 19 / TypeScript)
    ↓ Tauri IPC Commands
Application Layer (Rust tauri command handlers)
    ↓
Reconciliation Core (Rust domain logic, canonical models, multi-source matcher)
    ↓
Infrastructure (Local file reader, Calamine Excel parser, export engine)
```

## Current Phase
- **Phase 0 (Bootstrap)**: COMPLETED & READY
- **Phase 1 (Requirements & Data Contract)**: PENDING PROMPT 1

## Completed Components
- Repository structure and security configuration (`.gitignore`, `.editorconfig`).
- Long-term memory system in `.agent/` and `AGENTS.md`.
- Tauri 2 + React + TypeScript + Vite minimal bootstrap skeleton.
- Local build and test baselines (Vitest test suite, Rust cargo check/test).
- Environment setup (Rust 1.97.1 GNU toolchain, w64devkit GCC 16.2.0, WebView2 Runtime).

## Pending Components
- Multi-source data contract & canonical record models (Prompt 1).
- Accounting domain rules & profile definitions (Prompt 1+).
- Excel parsing engine with Calamine / streaming reader.
- Matching algorithm engine (exact, partial, aggregate, 1-N, N-M).
- Full UI presentation and reconciliation reporting views.
- Windows production packaging / MSI installer pipeline.

## Blockers
- None.

## Technical Debt / Risks
- None at bootstrap.

## Release Status
- Initial Bootstrap (v0.1.0-alpha.0).
