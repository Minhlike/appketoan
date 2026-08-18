# Architecture: Algorithmic Complexity & Multi-Key Indexing Strategy

## 1. Complexity Goals
For a large enterprise session containing $N = 100,000$ records across 3 to 4 workbooks:
- A naive nested-loop comparison exhibits quadratic complexity $O(N^2) \approx 10^{10}$ comparisons, taking tens of seconds to minutes.
- The Engine architecture guarantees linear $O(N)$ or near-linear $O(N \log N)$ execution time by utilizing multi-pass Hash Map indices and radix bucket partitions.

---

## 2. Index Structures

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Multi-Key Hash Indexes (Rust std::collections::HashMap / hashbrown)         │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. Exact Match Index:                                                       │
│    Key:   (normalized_series: String, normalized_doc_no: String)            │
│    Value: Vec<RecordIndex>                                                  │
│    Lookup Complexity: O(1)                                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. Partner + Amount Index:                                                  │
│    Key:   (normalized_tax_id: String, rounded_amount_vnd: i64)              │
│    Value: Vec<RecordIndex>                                                  │
│    Lookup Complexity: O(1)                                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. Aggregate Partition Bucket:                                              │
│    Key:   (normalized_tax_id: String, year_month: (i32, u32))               │
│    Value: Vec<RecordIndex>                                                  │
│    Lookup Complexity: O(1) per bucket, linear scan within small bucket      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Multi-Pass Matching Execution Workflow

```text
Input Records: Source A, Source B (and optional Sources C, D)
                  │
                  ▼
[Pass 1: Exact Key Matching - O(N)]
- Build Primary Index on Source B using (Series, DocNo)
- Iterate Source A: O(1) hash lookup against Source B
- If single candidate found AND TotalAmount matches exactly:
    -> Emit MATCHED_EXACT
    -> Mark records as consumed
                  │
                  ▼
[Pass 2: Exact Key with Amount/Tax Variance - O(K1)]
- Remaining unconsumed records with matching (Series, DocNo)
- Compare Pretax, VAT, and Total amounts:
    -> Emit MISMATCH_AMOUNT (detail: ERR_VAT_RATE_MISMATCH or ERR_AMOUNT_MISMATCH)
    -> Mark records as consumed
                  │
                  ▼
[Pass 3: Secondary Key Matching (Partner + Amount + Date Window) - O(K2)]
- For remaining records without standard docNo match (e.g. Bank Statements):
- Lookup by (TaxId, Amount) or (DateBucket, Amount) within tolerance window
- If unique candidate found:
    -> Emit MATCHED_WITH_TOLERANCE
    -> Mark records as consumed
- If multiple candidates found:
    -> Emit AMBIGUOUS_MATCH
                  │
                  ▼
[Pass 4: Aggregate Subset Sum Matching - O(B * 2^m)]
- For grouped records in small buckets (where bucket size m <= 8):
- Run dynamic programming / greedy subset sum to find 1-to-N or N-to-1 matches:
    -> Emit MATCHED_AGGREGATE
                  │
                  ▼
[Pass 5: Residual Unmatched Sweep - O(K3)]
- Records remaining unconsumed in Source A -> Emit UNMATCHED_MISSING_IN_TARGET
- Records remaining unconsumed in Source B -> Emit UNMATCHED_MISSING_IN_SOURCE
```

---

## 4. Performance Profile
- **Index Build Time**: $< 50\text{ ms}$ for 100k records.
- **Matching Pipeline Time**: $< 150\text{ ms}$ for 100k records.
- **Memory Footprint**: $< 120\text{ MB}$ RAM for complete session.
