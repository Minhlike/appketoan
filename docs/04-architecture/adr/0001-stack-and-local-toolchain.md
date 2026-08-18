# ADR 0001: Technology Stack and Local Compilation Strategy

## Status
Accepted

## Context
The application is a standalone desktop software running locally on Windows to reconcile financial and accounting data across multiple Excel workbooks. The application operates under strict security and confidentiality constraints:
1. 100% offline execution without cloud database, remote backend, or telemetry.
2. Low resource footprint and native Windows rendering.
3. High throughput for parsing large spreadsheets and executing multi-source reconciliation algorithms.
4. End users must not be required to install Node.js, Rust, Python, Docker, or database engines.

## Decision
1. **Application Shell**: Adopt **Tauri 2** with native Windows WebView2 integration.
2. **Frontend Layer**: Use **React 19 + TypeScript + Vite + Vanilla CSS**.
3. **Core Reconciliation Engine**: Implement in **Rust** as a local compiled module running inside the desktop binary.
4. **Local Windows Toolchain**: Use the `stable-x86_64-pc-windows-gnu` toolchain combined with `w64devkit` MinGW GCC / binutils. Configure `crate-type = ["staticlib", "rlib"]` for desktop builds to avoid GNU ld export ordinal limits.

## Consequences
- **Positive**: Blazing fast matching performance, native desktop look and feel, minimal memory consumption, zero external runtime dependency for end-users, guaranteed zero data exfiltration.
- **Negative**: Development requires local Rust compiler and MinGW toolchain (automated via setup scripts).
