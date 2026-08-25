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
