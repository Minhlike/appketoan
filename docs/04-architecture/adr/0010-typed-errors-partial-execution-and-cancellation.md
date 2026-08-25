# ADR 0010: Typed Errors, Partial Execution, and Cancellation

- Status: Accepted
- Date: 2026-08-26

## Context

An invalid workbook or failed control must not erase independent results or crash the desktop process. Long-running audits also need a safe stop path.

## Decision

Audit IPC returns `AuditExecutionError` with a code, scope, safe message, recoverability, and recommended action. Source ingestion failures are collected; planning continues with successfully prepared sources. Each ready control is isolated and produces `PASS`, `NEEDS_REVIEW`, `FAILED`, or `CANCELLED` independently.

Tauri runs the blocking audit on its blocking pool. An app-process cancellation registry owns a token per session. Cancellation is checked between source preparation and control boundaries. A cancelled ready control can never be presented as `PASS`, and session reset cancels pending work before releasing cache state.

## Consequences

- Recoverable source/control failures preserve usable evidence.
- Cancellation is cooperative, not preemptive inside every matcher loop.
- Legacy advanced execution now maps failures into the same safe typed envelope.
- More granular progress/cancellation checkpoints can be added without changing result semantics.
