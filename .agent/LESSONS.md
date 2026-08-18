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
