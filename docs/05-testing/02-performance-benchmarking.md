# Performance Engineering: Benchmarks & Capacity Targets

## 1. Capacity & Performance Targets

| Dataset Scale | Record Count | Max Parsing Time | Max Matching Time | Total Reconciliation | Peak RAM Budget |
|---|---|---|---|---|---|
| **Small (S)** | 1,000 rows | $< 50\text{ ms}$ | $< 10\text{ ms}$ | $< 100\text{ ms}$ | $< 50\text{ MB}$ |
| **Medium (M)** | 10,000 rows | $< 200\text{ ms}$ | $< 40\text{ ms}$ | $< 350\text{ ms}$ | $< 80\text{ MB}$ |
| **Large (L)** | 50,000 rows | $< 800\text{ ms}$ | $< 120\text{ ms}$ | $< 1.2\text{ s}$ | $< 150\text{ MB}$ |
| **Enterprise (XL)**| 100,000 rows | $< 1.5\text{ s}$ | $< 250\text{ ms}$ | $< 2.2\text{ s}$ | $< 200\text{ MB}$ |

---

## 2. Key Performance Metrics Measured
1. **Excel Ingestion Throughput**: Rows parsed per second via `calamine`. Target: $> 50,000\text{ rows/sec}$.
2. **Normalization Latency**: String sanitization and decimal conversion per 1,000 records. Target: $< 2\text{ ms}$.
3. **Multi-Key Hash Indexing**: Index generation time for dual-source sets. Target: $< 30\text{ ms}$ for 50k rows.
4. **Matching Engine Throughput**: Pairwise and multi-pass resolution per second. Target: $> 300,000\text{ records/sec}$.
5. **Memory Envelope**: Maximum resident set size (RSS) during full reconciliation cycle. Target: $\le 200\text{ MB}$.

---

## 3. Benchmark Harness Setup
The project includes a benchmark suite in `crates/reconciliation-core/benches/` using Criterion to continuously monitor performance regressions during development.
