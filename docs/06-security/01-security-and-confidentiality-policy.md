# Security Policy: Data Confidentiality & Zero Exfiltration

## 1. Core Security Mandate
Financial and tax records represent confidential business assets. The application is strictly engineered to operate in **Zero-Exfiltration Air-Gapped Mode**.

---

## 2. Hard Security Controls

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                       STRICT OFFLINE SECURITY BOUNDARY                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ [NO] Internet Connection Required for Any Business Feature                  │
│ [NO] Telemetry, Analytics, Usage Trackers, or Beacon Requests               │
│ [NO] Crash Reporting Over Network (Local logs only)                         │
│ [NO] Cloud Database or Remote API Backends                                  │
│ [NO] External AI / LLM API Endpoints (All logic is local deterministic code)│
│ [NO] Upload of User Excel Spreadsheets Anywhere Outside Local Memory/Disk   │
│ [NO] Dynamic External Script Ingestion (Strict Content Security Policy)     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Desktop Application Hardening
1. **Tauri Content Security Policy (CSP)**:
   - Restricts script and style origins strictly to `default-src 'self'`.
   - Blocks all outgoing network connections (`connect-src 'none'`).
2. **File System Scoping**:
   - Access is strictly limited to files explicitly opened or selected by the user via native Windows dialogs.
3. **Local Log Sanitization**:
   - Debug logs written locally must NEVER contain customer names, tax IDs, invoice numbers, or financial amounts.
   - Logs only record execution timing, record counts, and internal error codes (e.g. `PARSER_ROW_INVALID_FORMAT`).
4. **Temporary File Management**:
   - Temporary export files generated for preview are written directly to OS user temp directory with restricted permissions and purged on application close.
