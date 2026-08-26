# Lessons & Engineering Insights

## 1. Rust GNU Toolchain on Windows & DLL Export Limit
- **Problem**: When using `x86_64-pc-windows-gnu` toolchain on Windows, compiling a Tauri crate with `crate-type = ["cdylib"]` caused `ld.exe: error: export ordinal too large: 91385` because Windows DLLs have a 65535 (16-bit) maximum symbol export limit.
- **Solution**: For Windows desktop, `cdylib` is not required (it is only needed for Android/iOS mobile artifacts). Setting `crate-type = ["staticlib", "rlib"]` in `src-tauri/Cargo.toml` eliminates this issue and compiles cleanly.

## 2. MinGW w64devkit & libgcc_eh
- **Problem**: Rust GNU linker requests `-lgcc_eh`, which in newer GCC/w64devkit distributions is merged into `libgcc.a`.
- **Solution**: Creating a copy `libgcc_eh.a` of `libgcc.a` in the GCC library directory allows the linker to resolve all runtime exception handling symbols seamlessly.

## 3. Cargo Sparse Index Protocol
- **Problem**: Default git-based crates index download clones ~500MB of history which can take minutes on Windows.
- **Solution**: Configuring `protocol = "sparse"` in `.cargo/config.toml` downloads package metadata via fast HTTP endpoints in seconds.

## 4. Vitest Execution Pool on Windows (Node 24)
- **Problem**: Vitest default worker pool can experience hang states on some Windows environments with Node 24.
- **Solution**: Specifying `pool: "forks"` in `vite.config.ts` ensures rapid, reliable test runs.

## 5. Bound Before Exponential Enumeration
- **Problem**: Computing `1usize << candidate_count` before proving the candidate set is within budget can overflow or trigger infeasible subset work. A unique direct match also makes subset enumeration unnecessary.
- **Solution**: Complete the O(n) direct scan first. Return ambiguous for multiple direct matches, accept one direct match immediately, and only compute subset bounds when there are zero direct matches and the candidate count is within the fixed aggregate budget.

## V16 Period and Local Acceptance Traps (2026-08-25)
- A single audit package may contain controls whose evidence belongs to different sub-periods. Keep one explicit session boundary, then calculate the intersection of required-source date evidence separately for each control; never widen a narrow source to match a broader workbook.
- Office lock files and exact duplicate derived workbooks can appear beside an acceptance set. Ignore lock files and collapse only proven same-kind content duplicates in a local harness. Production planning must leave unrelated duplicate capabilities as `NEEDS_REVIEW` rather than choosing by load order or filename.

## V17 GNU Tauri Test Harness Boundary (2026-08-26)
- Adding managed Tauri runtime state can make the synthetic library test harness load WebView2 symbols and fail before running tests with `STATUS_ENTRYPOINT_NOT_FOUND` on the supported GNU toolchain.
- Keep accounting and resilience unit/integration tests in the pure Rust core. The Tauri shell has no lib unit tests, so its synthetic lib/doctest target is disabled; validate that boundary with workspace check, strict Clippy, production Tauri build, executable smoke, frontend tests, and static privacy/IPC contract tests.

## V18 Document-Completeness Boundary (2026-08-26)
- Dropping unparseable-date rows during period filtering makes later field validation impossible. Retain them in a review-only collection that is never indexed for matching.
- A required-source date intersection is not safe for a completeness control: it can hide the missing document that caused a source to start late or end early. Filter once to the explicit user period, keep source coverage as warning evidence, and compare the entire selected period. Bank reconciliation retains its distinct fail-closed intersection policy.
- When a typed result is authoritative, do not materialize a second full legacy result in the same default pipeline. Preserve compatibility through an explicit legacy path or a lazy adapter, not duplicate execution.

## V18 Rust-to-TypeScript Collection Contract (2026-08-26)
- Do not combine `skip_serializing_if` with a frontend field declared as required. An empty Rust map/vector then disappears from IPC JSON and ordinary frontend collection operations can crash the entire WebView.
- Keep required wire collections present even when empty, add a serialization-shape regression, and still normalize at the rendering boundary so an older or partial payload fails closed instead of replacing the whole application with an error screen.
- Rendered-UI smoke needs a stable ASCII readiness marker. Human-language UI text can be represented inconsistently by Windows accessibility APIs even when the visible page is correct.

## V18 Partial Evidence Is Not Partial PASS (2026-08-26)
- A multi-source control should declare both its minimum executable evidence and its complete assurance evidence. Refusing to run when one later-stage source is absent hides valid checks; silently making that source optional can create a false PASS.
- Run the independent available comparison, preserve missing capabilities, mark unavailable fields `NOT_CHECKED`, and require complete evidence before any overall PASS.
- Keep inventory-level missing controls in the Control Plan. Put only loaded, actionable controls in the Review Queue so absent unrelated workflows do not look like failures of the current dossier.
