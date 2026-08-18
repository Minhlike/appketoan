# Architectural Decision Log

## DEC-0001: Technology Stack Selection (Tauri 2 + Rust + React + TS)
- **Date**: 2026-08-18
- **Context**: The product is an offline-first Windows desktop application processing sensitive, multi-source accounting Excel workbooks locally without servers, cloud databases, or containerization.
- **Decision**: Adopt Tauri 2 with Rust for the core local backend and React + TypeScript + Vite for the UI.
- **Status**: ACCEPTED (See [ADR-0001](file:///D:/appketoan/docs/04-architecture/adr/0001-stack-and-local-toolchain.md))

## DEC-0002: Toolchain Selection on Windows (x86_64-pc-windows-gnu with MinGW w64devkit)
- **Date**: 2026-08-18
- **Context**: The developer environment lacked MSVC C++ Build Tools (`link.exe`).
- **Decision**: Configure Rust toolchain `stable-x86_64-pc-windows-gnu` and MinGW GCC / dlltool via `w64devkit`, and set `crate-type = ["staticlib", "rlib"]` in `src-tauri/Cargo.toml`.
- **Status**: ACCEPTED

## DEC-0003: Multi-Source Architecture First
- **Date**: 2026-08-18
- **Context**: The reconciliation engine must support arbitrary numbers of data sources (e.g. 1-to-1, 1-to-N, N-to-M, bank vs invoice vs ledger vs branch files).
- **Decision**: Architectural boundary is structured around generic `DataSource[]`, `ReconciliationSession`, `CanonicalRecord`, and `MatchingRule` concepts. Hardcoded binary comparison assumptions (`file1`/`file2`) are strictly prohibited.
- **Status**: ACCEPTED

## DEC-0004: Standalone Domain Crate Architecture (`crates/reconciliation-core`)
- **Date**: 2026-08-18
- **Context**: Keeping domain models and matching algorithms coupled inside `src-tauri` forced test executables to link heavy GUI/WebView2 libraries, slowing test cycles and creating unwanted platform dependencies.
- **Decision**: Extract domain logic and canonical models into a dedicated, standalone pure-Rust library crate `crates/reconciliation-core`. `src-tauri` consumes it via Cargo path dependency.
- **Rationale**: Clean separation of concerns, blazing-fast standalone unit test runs (< 0.01s), zero GUI coupling.
- **Status**: ACCEPTED
