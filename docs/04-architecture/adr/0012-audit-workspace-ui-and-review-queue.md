# ADR 0012: Audit Workspace UI and Unified Review Queue

- Status: Accepted
- Date: 2026-08-26

## Context

The default product workflow had exposed scenario and engine terminology to accounting users while `App.tsx` owned unrelated source, execution, review, and legacy state.

## Decision

The default desktop flow is organized as four visible steps: load the accounting package, select the period, run eligible controls, and review findings. `AuditWorkspacePage`, source intake, period, planner/result, and review queue components own presentation; `useAuditExecution` owns typed audit operation state.

Control/source labels use Vietnamese accounting language. Suggested and ambiguous bank links remain review items and are never rendered as accepted. Status uses text and shape/ARIA, not color alone. Stable source/control/finding identities are used for navigation keys. The advanced scenario workflow remains available behind an explicit disclosure panel.

Tauri drag/drop uses native local paths. Browser/file-picker ingestion keeps the byte IPC fallback. Exact byte duplicates and Office lock files are reported instead of silently creating logical sources.

## Consequences

- The normal workflow does not require knowledge of `SourceRole` or engine semantics.
- Review conditions are consolidated and actionable without converting acknowledgement into accounting evidence.
- Existing advanced behavior remains available during migration.
- Backend progress events and record-level navigation are future additive work.
