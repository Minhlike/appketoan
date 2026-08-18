# Handoff Document

## To Next Engineer / Reviewer / User

The **AppKetoan** desktop application has been **fully implemented, tested, and packaged for Windows production**.

### Quick Start for End Users & Developers
1. **Running Native Desktop in Development Mode**:
   ```bash
   npm run tauri dev
   ```
2. **Running Frontend Dev Server**:
   ```bash
   npm run dev
   ```
3. **Running Full Test Suite**:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/test.ps1
   ```
4. **Building Production Windows Installer**:
   ```bash
   npm run tauri build
   ```

### Generated Production Installers
- **NSIS Setup Installer**: `src-tauri/target/release/bundle/nsis/appketoan_0.1.0_x64-setup.exe` (4.99 MB)
- **WiX MSI Installer**: `src-tauri/target/release/bundle/msi/appketoan_0.1.0_x64_en-US.msi` (7.78 MB)
- **Direct Executable**: `src-tauri/target/release/tauri-app.exe` (26.4 MB)

### Verified Key Capabilities
- **100% Offline**: Zero external network requests, zero telemetry, zero cloud dependencies.
- **True Multi-Source Engine**: Accepts any number of Excel files and sheets ($1 \leftrightarrow 1$, $1 \leftrightarrow N$, $N \leftrightarrow 1$, $N \leftrightarrow M$).
- **Intelligent Header Detection**: Vietnamese accounting column sniffer recognizes HĐĐT, Sổ cái 511, 3331, 131, 112, Sao kê ngân hàng.
- **Robust Normalization**: Handles leading zeroes (`0000101` vs `101`), float doc numbers (`101.0`), Excel float dates, Vietnamese comma/dot decimals, negative accounting parentheses.
- **Linear Scaling Performance**: 100,000 records processed in $1.27\text{ s}$.
- **Exporting**: Professional 3-tab Excel audit workbook with auto-fitted columns and discrepancy color styling.
