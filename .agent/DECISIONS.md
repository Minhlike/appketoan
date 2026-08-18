# Architectural Decision Log

## DEC-0001: Technology Stack Selection (Tauri 2 + Rust + React + TS)
- **Date**: 2026-08-18
- **Context**: The product is an offline-first Windows desktop application processing sensitive, multi-source accounting Excel workbooks locally without servers, cloud databases, or containerization.
- **Decision**: Adopt Tauri 2 with Rust for the core local backend (reconciliation engine, Excel parser, validation) and React + TypeScript + Vite for the UI.
- **Rationale**: Minimal memory and bundle footprint, native Windows integration via WebView2 Runtime (built into Windows 11), high-performance parallel data processing in Rust, zero telemetry/cloud lock-in.
- **Trade-offs**: Native compilation required during development (managed via Rust GNU toolchain and MinGW w64devkit).
- **Status**: ACCEPTED (See [ADR-0001](file:///D:/appketoan/docs/04-architecture/adr/0001-stack-and-local-toolchain.md))

## DEC-0002: Toolchain Selection on Windows (x86_64-pc-windows-gnu with MinGW w64devkit)
- **Date**: 2026-08-18
- **Context**: The developer environment lacked MSVC C++ Build Tools (`link.exe`).
- **Decision**: Configure Rust toolchain `stable-x86_64-pc-windows-gnu` and MinGW GCC / dlltool via `w64devkit`, and set `crate-type = ["staticlib", "rlib"]` in `src-tauri/Cargo.toml` to prevent GNU `ld.exe` export ordinal overflow on Windows desktop.
- **Rationale**: Enables standalone local compilation and testing without requiring heavy Visual Studio installations.
- **Trade-offs**: MinGW-w64 runtime is used instead of MSVC runtime.
- **Status**: ACCEPTED

## DEC-0003: Multi-Source Architecture First
- **Date**: 2026-08-18
- **Context**: The reconciliation engine must support arbitrary numbers of data sources (e.g. 1-to-1, 1-to-N, N-to-M, bank vs invoice vs ledger vs branch files).
- **Decision**: Architectural boundary is structured around generic `DataSource[]`, `ReconciliationSession`, `CanonicalRecord`, and `MatchingRule` concepts. Hardcoded binary comparison assumptions (`file1`/`file2`) are strictly prohibited.
- **Rationale**: Future-proof against complex accounting workflows across multiple branches and periods.
- **Status**: ACCEPTED
