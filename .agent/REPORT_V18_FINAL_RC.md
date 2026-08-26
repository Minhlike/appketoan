# V18 Final Correctness Fix and Release Candidate

## Scope

- Branch: `codex/v18-document-integrity`
- Pull request: `https://github.com/Minhlike/appketoan/pull/4`
- Merge status: not merged
- No new account feature, telemetry, network path, workbook, or binary artifact was added to source control.

## Correctness blocker closure

### Accounting-period boundary

`REVENUE_INVOICE_REGISTER_LEDGER` now treats the user-selected `AccountingPeriod` as the authoritative completeness boundary. Source earliest/latest dates remain catalog evidence and warnings only; they never shrink the document reconciliation range. Synthetic regressions cover missing BK evidence on the first and last day, missing TK511 evidence at a boundary, and late-start/early-end sources. The independent bank control retains its intersection-based fail-closed policy.

The ignored local acceptance run exposed boundary evidence previously hidden by the source intersection: 66 BK records at the beginning of the selected period are now reported as `EXTRA_IN_BK`, not silently excluded. This count is observed evidence, not a hard-coded oracle.

### Invoice lifecycle

The document evaluator reuses the existing typed lifecycle evaluator. A mapped adjusted, affected-by-adjustment, replaced, cancelled, or unknown lifecycle adds `INVOICE_LIFECYCLE_NEEDS_REVIEW` with high severity. Exact date, number, money, BK, and TK511 evidence cannot convert those invoices to `FULLY_MATCHED` or control `PASS`.

## Performance evidence

The reviewed V18 gate recorded 78.20 seconds. A same-machine pre-fix reproduction during this task completed in 13.43 seconds cold total and 12.33 seconds warm execution; observed peak working set was 2,355,978,240 bytes (2,246.8 MiB).

After removing the duplicate generic revenue reconciliation and borrowing prepared period-filtered datasets when possible:

- V15 tri-source reference: 5.46 seconds.
- V18 prepare: 1.14 seconds.
- V18 cold execution: 6.67 seconds; revenue stage 6.17 seconds.
- V18 cold total: 7.80 seconds.
- V18 warm execution: 7.44 seconds; revenue stage 7.14 seconds.
- Observed peak working set: 1,365,471,232 bytes (1,302.2 MiB), 42.0% below the reproduced pre-fix peak.

This is a one-iteration debug benchmark, not median/p95. The peak is a sampled test-process working set and includes the benchmark's retained warm audit clone.

## Acceptance and regression

- Real local immutable record-count, exact-match, missing-document, and gross-variance oracles: PASS.
- #233: `NEEDS_REVIEW` / `HIGH`; present in Invoice and BK, missing in TK511.
- Real BK invalid-date evidence remains review-only; document totals remain `NOT_VERIFIED`.
- Real bank classification remains conserved: 225 parsed, zero strong accepted, Suggested/Ambiguous review-only, and no review link consumed as accepted.
- Partner identity and sales-analysis layer controls: PASS.
- Duplicate BK, amount-only prohibition, exact five-field Thuế/BK, independent totals, TK511 `NOT_CHECKED`, typed errors, partial failure, cancellation, cache reuse/invalidation, privacy, and Review Queue invariants: PASS.

## UI and test gates

- `cargo test --workspace`: PASS; 134 executed tests, two confidential harnesses ignored by default.
- Both ignored local acceptance harnesses: PASS.
- `npm run test`: PASS; 29/29 across eight files.
- `npm run typecheck`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `npx tauri build`: PASS.
- Fresh release executable smoke after five seconds: PASS.

UI tests cover summary/detail, side-by-side evidence, simultaneous typed document errors, `NOT_CHECKED`, partial/failed Review Queue behavior, and cancelled workspace/control labels without PASS wording. Automated browser visual inspection remains unverified.

## Local-only RC artifacts

- Portable: `target/release/appketoan.exe` — SHA256 `02E3572FC9770EE5647BA372BCA52E1442E1A9EC8F9955BEF3956C20574F4A8C`
- NSIS: `target/release/bundle/nsis/appketoan_0.1.0_x64-setup.exe` — SHA256 `7C26EF8AC7CE3D415E2E4442E26577E7420A841F3F7B59C72725961E512635D9`
- MSI: `target/release/bundle/msi/appketoan_0.1.0_x64_en-US.msi` — SHA256 `AC3FAEDF0D45F9D05451387254D410237A1BE753F25F488093FE55C83D9D3824`

All three artifacts were regenerated after the recorded build start and remain ignored by Git.

## Known limitations

- One local BK row has no parseable required date, so affected period totals remain `NOT_VERIFIED`.
- TK511 VAT and receivable remain intentionally `NOT_CHECKED`.
- Audit Workspace revenue no longer materializes the optional legacy `ReconciliationResult`; the advanced legacy scenario path remains unchanged.
- Cancellation remains cooperative at source/control boundaries rather than inside every evaluator loop.
- Automated browser visual inspection and production-process median/p95 memory are not verified.

No correctness blocker remains for final ChatGPT review.

## Portable EXE startup correction — superseding smoke evidence

The prior five-second process-only smoke was invalidated after a live portable run displayed an empty WebView. The source and embedded frontend asset were both present, but the desktop shell exposed the window before frontend readiness and had no visible bootstrap failure surface. The release workflow also had no assertion that React content actually rendered.

The correction is limited to the portable desktop startup path:

- keep the Tauri window hidden until the local page-load completion event;
- show a local startup indicator and a fail-closed bootstrap error instead of an empty root;
- report React uncaught bootstrap errors through the visible fallback;
- verify the Windows UI Automation tree rather than process liveness.

Final EXE-only evidence:

- `cargo test --workspace`: PASS.
- `npm run test -- --run`: PASS, 29/29.
- `npm run typecheck`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `npx tauri build --no-bundle`: PASS.
- Rendered UI smoke, normal profile: PASS, 72 UI elements.
- Rendered UI smoke, fresh WebView2 profile: PASS, 72 UI elements.
- Direct Windows window capture: PASS; full dashboard visible.
- Portable: `target/release/appketoan.exe` — SHA256 `CB38798F06A2F367B5D6B950B18B33886A25E4DACCBF8365A74747F5AA8D7F2B`.

NSIS and MSI were intentionally not rebuilt because this correction was explicitly restricted to the portable EXE. Their earlier hashes do not describe the corrected executable.
