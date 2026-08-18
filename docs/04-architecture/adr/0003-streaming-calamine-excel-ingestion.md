# ADR 0003: Streaming Excel Ingestion with Calamine

## Status
Accepted

## Context
Accounting workbooks may contain tens of thousands of rows across multiple sheets, reaching tens of megabytes. Using heavy OLE/COM bridges, Python subprocesses, or unoptimized Node.js sheet parsers degrades UI responsiveness and risks out-of-memory errors on client Windows machines.

## Decision
1. Ingest all Excel files (`.xlsx`, `.xls`, `.xlsb`, `.xlsm`) using the pure-Rust `calamine` crate.
2. Read worksheets using streaming row iterators, immediately converting raw cell values into sanitized `CanonicalRecord` structs.
3. Drop raw sheet matrices from memory immediately after extraction to keep RAM usage under 150MB.

## Consequences
- **Positive**: Blazing fast parsing (> 50,000 rows/second), zero external dependencies (no MS Excel installation required), low memory usage.
- **Negative**: Formatted cell charts or embedded macro code are omitted (not needed for accounting reconciliation).
