# Quality Assurance: Testing Strategy & Verification Pipeline

## 1. Test Pyramid & Layers

```text
               / \
              /   \
             / E2E \       Level 4: End-to-End Desktop Flow (Tauri WebDriver)
            /-------\
           / Golden  \     Level 3: Golden Datasets & Synthetic Accounting Fixtures
          /-----------\
         / Integration \   Level 2: Engine Multi-Pass Matching & Parser Tests
        /---------------\
       /   Unit Tests    \ Level 1: Normalizers, Math Invariants, Serializers
      /-------------------\
```

---

## 2. Testing Levels Breakdown

### Level 1: Unit Tests (Rust & TypeScript)
- Fast, isolated checks for utility functions:
  - Document number sanitization (`0000123` $\rightarrow$ `123`, `123.0` $\rightarrow$ `123`).
  - Tax ID filtering (`MST: 0101234567` $\rightarrow$ `0101234567`).
  - Monetary invariant computation ($\text{Total} = \text{Pretax} + \text{VAT}$).
  - Serde JSON serialization roundtrips.

### Level 2: Integration Tests (Rust Core)
- Feed multi-source sets of `CanonicalRecord` through `reconciliation-core::matcher`.
- Verify accurate assignment of all `MatchStatus` states:
  - Exact match ($1 \leftrightarrow 1$)
  - Tolerance match ($|\Delta| \le \epsilon$)
  - Aggregate match ($1 \leftrightarrow N, N \leftrightarrow 1, N \leftrightarrow M$)
  - Discrepancy detection (amount, tax rate, date, account)
  - Missing in source vs missing in target
  - Duplicate suspects and ambiguous candidates.

### Level 3: Golden Dataset Verification
- Standardized fixture files in `fixtures/synthetic/` processed against `fixtures/expected/`.
- Automated regression suite asserts exact match of summary metrics and match group lists.

### Level 4: Performance & Load Benchmarks
- Synthetic datasets scaling from 1,000 to 100,000 rows.
- Automated benchmarks verify time and memory constraints.

---

## 3. Strict Prohibitions
- **NO REAL ACCOUNTING DATA**: Real company financial records, customer tax codes, or actual invoice spreadsheets must NEVER be committed to version control.
- **NO FAKE ASSERTIONS**: Tests must evaluate real functional logic and realistic assertions.
