# Testing Strategy & Quality Assurance

## Strategy
1. **Unit Testing**: Vitest for frontend components; `cargo test` for Rust domain logic and math.
2. **Golden / Synthetic Testing**: Synthetic fixtures in `fixtures/synthetic/` with expected outputs in `fixtures/expected/`.
3. **End-to-End Testing**: Validating workbook parsing, reconciliation flow, and reporting export.

## Execution
- Frontend: `npm test`
- Typecheck: `npm run check`
- Backend: `powershell -File scripts/cargo-test.ps1`
