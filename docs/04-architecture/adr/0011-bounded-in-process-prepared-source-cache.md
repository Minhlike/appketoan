# ADR 0011: Bounded In-Process Prepared Source Cache

- Status: Accepted
- Date: 2026-08-26

## Context

Changing a period or rerunning eligible controls should not parse and normalize unchanged workbooks again. Persisting confidential workbook content or normalized records to disk would introduce a new privacy boundary.

## Decision

V17 implements only an in-memory L1 cache. A prepared source key contains stable source identity, raw SHA-256, sheet, header/data-row and mapping fingerprint, source kind, and normalization schema version. Filename is not identity. Accounting period is intentionally excluded from the normalization key: every execution rebuilds period-filtered views and indexes from the unchanged normalized dataset.

The cache is bounded to 32 entries and a conservative 1 GiB estimated footprint. A source is bounded to 256 MiB. Source removal retains only active keys; session reset releases all unreferenced keys. No raw or normalized accounting content is persisted by the cache.

## Consequences

- Same bytes/sheet/mapping/schema reuse normalization within the app process.
- Changed bytes, mapping, sheet, kind, or schema invalidate safely.
- Period changes reuse normalization but never reuse the previous period view.
- Current path ingestion hashes and parses from an in-memory byte buffer; streaming parse is deferred.
