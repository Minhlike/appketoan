# APPKETOAN — Offline Multi-Source Accounting Reconciliation Desktop Platform

An offline-first Windows desktop application designed to perform automated reconciliation of accounting data across multiple Excel workbooks and sheets (electronic invoices, general ledgers, cash books, tax accounts, bank statements, multi-branch books, and custom formats).

---

## Key Principles & Architectural Guarantees

1. **100% Offline-First**: All data parsing, validation, matching, and report generation operate strictly on the local machine.
2. **Zero Exfiltration**: No telemetry, no analytics, no cloud databases, and no remote API calls. Financial data never leaves the local environment.
3. **Multi-Source by Design**: Built from the ground up to support arbitrary sources ($1 \leftrightarrow 1$, $1 \leftrightarrow N$, $N \leftrightarrow 1$, $N \leftrightarrow M$) without hardcoded two-file assumptions.
4. **Zero End-User Runtime Dependencies**: Packaged as a standalone Windows installer; end users do not need Node.js, Rust, Python, Docker, or database engines.

---

## Technology Stack

- **Application Shell**: Tauri 2 (Windows WebView2 Runtime)
- **Local Reconciliation Core**: Rust (stable-x86_64-pc-windows-gnu with MinGW GCC / binutils)
- **Presentation Layer**: React 19 + TypeScript + Vite
- **Styling**: Vanilla CSS (Function-Driven Design)
- **Testing**: Vitest (Frontend) & Cargo Test (Backend)

---

## Repository Structure

```text
D:\appketoan
├── AGENTS.md                   # AI Agent operating workflow & mandatory protocols
├── README.md                   # Project overview & developer guide
├── .gitignore                  # Strict security & confidentiality filters
├── .editorconfig               # Formatting standards
│
├── .agent/                     # Long-term persistent AI memory
│   ├── PROJECT_STATE.md        # Current state snapshot
│   ├── CURRENT_TASK.md         # Active task scope & checklist
│   ├── DECISIONS.md            # Architectural decision logs
│   ├── HANDOFF.md              # Cross-agent handoff notes
│   ├── LESSONS.md              # Engineering insights & traps
│   └── TEST_STATUS.md          # Quality assurance & test dashboard
│
├── docs/                       # Project documentation
│   ├── 01-requirements/       # Business requirements & user stories
│   ├── 02-accounting-rules/   # Accounting scenario rules & math
│   ├── 03-data-contract/      # Canonical data schema & contracts
│   ├── 04-architecture/       # Architecture design & ADRs
│   │   └── adr/
│   ├── 05-testing/            # Testing strategy & QA guides
│   ├── 06-security/           # Security & data confidentiality rules
│   └── 07-release/            # Packaging & installer pipeline
│
├── fixtures/                   # Test datasets
│   ├── synthetic/              # Synthetic dummy data only (NO REAL DATA)
│   └── expected/               # Expected golden outputs
│
├── scripts/                    # PowerShell development & test scripts
│   ├── check.ps1               # Typecheck and cargo check
│   ├── test.ps1                # Run frontend and backend tests
│   ├── build.ps1               # Build frontend and backend binaries
│   └── dev.ps1                 # Start local development server
│
├── src/                        # React / TypeScript UI source
├── src-tauri/                  # Rust Tauri 2 desktop backend source
├── tests/                      # Integration test suites
└── releases/                   # Production distribution artifacts
```

---

## Development & Verification Commands

### Prerequisites
- Node.js >= 20, npm >= 10
- Rust >= 1.97.1 (`x86_64-pc-windows-gnu` or `x86_64-pc-windows-msvc`)
- MinGW-w64 toolchain (e.g. `w64devkit` in `D:\DevTools\w64devkit`)
- Windows 10/11 with WebView2 Runtime

### Common Commands
```powershell
# 1. Run full type and compilation checks
powershell -ExecutionPolicy Bypass -File scripts/check.ps1

# 2. Run all baseline test suites
powershell -ExecutionPolicy Bypass -File scripts/test.ps1

# 3. Build frontend and backend artifacts
powershell -ExecutionPolicy Bypass -File scripts/build.ps1

# 4. Launch development server
npm run dev

# 5. Launch Tauri desktop app in development
npm run tauri dev
```

---

## STRICT SECURITY WARNING: CONFIDENTIAL FINANCIAL DATA

> [!CAUTION]
> **DO NOT COMMIT REAL ACCOUNTING DATA OR WORKBOOKS.**
> Real company workbooks (`.xlsx`, `.xls`, `.csv`), customer statements, tax files, and invoices contain sensitive financial secrets.
> Only synthetic/mock data generated from scratch is permitted inside `fixtures/synthetic/`.
