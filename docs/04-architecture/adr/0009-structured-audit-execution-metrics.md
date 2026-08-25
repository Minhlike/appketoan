# ADR 0009: Structured Audit Execution Metrics

- Status: Accepted
- Date: 2026-08-26

## Context

A single elapsed-time log cannot distinguish workbook I/O, parsing, normalization, planning, control execution, serialization, IPC overhead, and rendering. It also invites comparisons between different workloads.

## Decision

`AuditExecutionMetrics` is part of the typed audit report. Backend stages and per-control durations are measured separately. The frontend records IPC-inclusive time, conservative round-trip overhead (`IPC inclusive - backend`), and the first scheduled render after a report arrives.

Metrics identify workload shape and whether a run is cold or reused. `peakMemoryBytes` remains optional and unset until a reliable Windows process measurement is added. Benchmark reports must state build profile, record count, control count, and iteration count; a single iteration is not called median or p95.

## Consequences

- Performance evidence is machine-readable and travels with the report.
- Backend and UI timing can evolve without parsing text logs.
- Round-trip overhead includes command scheduling and is not claimed as pure serialization time.
- Current peak memory remains explicitly unverified.
