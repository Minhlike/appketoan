# Release Engineering: Windows Packaging & Distribution

## 1. Zero-Prerequisite Delivery Model
End users must be able to install and run the application immediately without installing:
- No Node.js / npm
- No Rust / Cargo
- No Python / pip
- No Docker / Virtual Machines
- No Git / Terminal Tools
- No Database Engines (MySQL, Postgres, SQL Server)

The distribution package is a standalone single-file installer or portable binary:
- **NSIS Installer**: `appketoan_setup_x64.exe`
- **Windows Installer (MSI)**: `appketoan_x64.msi`
- **Portable Binary**: `appketoan_portable_x64.zip`

---

## 2. Windows Runtime Dependencies
The application relies exclusively on the **Microsoft Edge WebView2 Runtime**, which is pre-installed on all Windows 11 systems and modern Windows 10 updates (Version 2004+). If WebView2 is absent on legacy Windows 10 installations, the NSIS bootstrapper provides an embedded offline evergreen bootstrapper.

---

## 3. Production Build Pipeline
```powershell
# 1. Verify all test suites pass
powershell -ExecutionPolicy Bypass -File scripts/test.ps1

# 2. Build production web bundle
npm run build

# 3. Build optimized Windows desktop executable & installer
npm run tauri build
```

---

## 4. Release Checklist
- [ ] Version updated in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- [ ] Automated unit and golden dataset tests pass with 0 warnings.
- [ ] Security audit confirms zero network telemetry or tracking packages.
- [ ] Installer artifact validated on a clean Windows 10/11 virtual machine without developer tools.
- [ ] Artifacts signed and archived in `releases/`.
