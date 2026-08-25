# HANDOFF: AppKetoan — Agent Tiếp Quản Phát Triển

> **Ngày bàn giao gốc:** 2026-08-25
> **Mục đích tài liệu:** Single Source of Truth cấp handoff cho agent tiếp quản dự án.
> **Ngôn ngữ làm việc:** Tiếng Việt.
> **Đối tượng sử dụng cuối:** Người làm kế toán Việt Nam, ưu tiên cả người không rành kỹ thuật.
> **Nguyên tắc:** Không suy đoán trạng thái repo. Mọi trạng thái build/test/git phải được agent hiện tại xác minh lại bằng lệnh thực tế.

---

## 0. ĐỌC PHẦN NÀY TRƯỚC

AppKetoan không được phát triển thành một ERP/MISA thu nhỏ nếu chưa có yêu cầu rõ ràng.

**Định vị sản phẩm ưu tiên:**

> AppKetoan = Automated Accounting Reconciliation & Control
> Nhận dữ liệu từ Excel/sổ kế toán/hóa đơn/sao kê → chuẩn hóa → đối chiếu → chỉ ra sai lệch có bằng chứng.

Core phải:
- deterministic;
- explainable;
- fail-closed;
- ưu tiên false-negative hơn false-positive trong kết luận kế toán;
- giữ provenance của dữ liệu;
- không dùng LLM/AI để quyết định một giao dịch có “khớp” hay không.

AI, nếu bổ sung sau này, chỉ được dùng để:
- giải thích lỗi bằng tiếng Việt;
- gợi ý cách xử lý;
- hỗ trợ mapping ban đầu;
- KHÔNG được tự xác nhận bút toán hay che giấu thiếu bằng chứng.

---

## V15 LOCAL DATASET EXPANSION — 2026-08-25

- Branch review target: `feature/v15-real-accounting-datasets` (not merged to `main`).
- The user confirmed the seven suffixless files in `.local-testdata/` are the local acceptance set. The directory is ignored; never commit or transmit those workbooks.
- The engine now distinguishes ledger 112, partner masters, sales registers, sales-analysis reports, and bank statements; support evidence is sanitized in `docs/REAL_DATASET_SUPPORT_MATRIX.md`.
- Preserve the baseline automated acceptance contract. V15 adds direction compatibility for ledger-112/bank comparisons and refuses to match equal amounts with contradictory directions.

---

## 1. BASELINE BẤT BIẾN

Dataset baseline:

```text
T7.2026 Thuế.xlsx + T7.2026.xlsx

Hóa đơn hợp lệ:        46
Bản ghi TK511 hợp lệ:  45
Khớp chính xác:        45
Thiếu ở TK511:         #233
Doanh thu chưa thuế:   105.000.000 VNĐ
```

Viết gọn:

```text
46 / 45 / 45 / #233 / 105M
```

**Không được merge/release bất kỳ thay đổi nào làm phá baseline này nếu chưa chứng minh baseline cũ sai và có phê duyệt thay đổi nghiệp vụ.**

Baseline phải được kiểm tra bằng automated regression test, không chỉ kiểm tra UI.

---

## 2. PHẠM VI SẢN PHẨM

### 2.1. Phạm vi hiện tại được ưu tiên

1. Đọc dữ liệu kế toán từ file.
2. Nhận diện loại nguồn.
3. Chuẩn hóa dữ liệu.
4. Đối chiếu nhiều nguồn.
5. Phát hiện:
   - thiếu bản ghi;
   - thừa bản ghi;
   - lệch tiền;
   - lệch metadata;
   - trùng;
   - ambiguous match;
   - aggregate match;
   - dữ liệu chưa đủ bằng chứng.
6. Xuất báo cáo có thể truy ngược về nguồn gốc dữ liệu.
7. Chạy offline trên Windows.

### 2.2. Các workflow mục tiêu

```text
A. Hóa đơn thuế  ↔ TK511          Revenue
B. Hóa đơn thuế  ↔ TK3331         VAT
C. Hóa đơn thuế  ↔ TK131          Receivable
D. TK112          ↔ Sao kê NH     Bank reconciliation
E. DM khách hàng  ↔ Hóa đơn/Sổ    Identity validation
F. Bảng kê        ↔ Hóa đơn/Sổ    Cross-source control
```

Mỗi workflow phải có rule riêng. Không dùng một bộ tolerance/global rule để quyết định tất cả nghiệp vụ.

### 2.3. Không thuộc phạm vi mặc định

Không tự động mở rộng sang:
- lập chứng từ kế toán hoàn chỉnh;
- tự động hạch toán vào sổ thật;
- khai thuế;
- ký/phát hành hóa đơn điện tử;
- ERP đầy đủ;
- payroll;
- cloud accounting;
- tự động sửa file nguồn của người dùng.

Nếu cần các chức năng trên, phải tạo ADR/feature proposal riêng.

---

## 3. STACK

- Tauri 2 — desktop shell
- Rust — engine `reconciliation-core`
- React 19 + TypeScript + Vite — frontend
- Toolchain: `x86_64-pc-windows-gnu`
- GNU toolchain, KHÔNG tự ý chuyển MSVC

Money rules:
- Rust: `Decimal`
- TypeScript: `string`
- KHÔNG dùng `f32`, `f64`, JS `number` cho giá trị tiền nghiệp vụ.

---

## 4. TOOLCHAIN PATH

```powershell
$env:PATH = "C:\Program Files\Git\cmd;D:\DevTools\w64devkit\bin;C:\Users\Acer\.cargo\bin;D:\DevTools\nodejs;D:\DevTools\npm-global;" + $env:PATH
```

Repo:

```text
D:\appketoan
```

---

## 5. CẤU TRÚC REPO

```text
D:\appketoan\
├── Cargo.toml
├── crates/
│   ├── reconciliation-core/
│   │   ├── src/
│   │   │   ├── models/
│   │   │   ├── reader/
│   │   │   ├── normalizer/
│   │   │   ├── intake/
│   │   │   ├── matcher/
│   │   │   ├── analyzer/
│   │   │   └── exporter/
│   │   └── tests/
│   └── webview2-com-sys/
├── src-tauri/
│   └── src/commands.rs
├── src/
│   ├── components/
│   ├── types/dataContract.ts
│   ├── services/api.ts
│   └── utils/money.ts
└── docs/
```

`reconciliation-core` phải tiếp tục là Rust core độc lập, không phụ thuộc Tauri.

UI/Tauri không được chứa nghiệp vụ reconciliation mà core có thể sở hữu.

---

## 6. LỆNH KIỂM TRA CHUẨN

```powershell
cd D:\appketoan

git status
git diff --stat
git diff

cargo test --workspace
npm run test
npm run typecheck
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

npx tauri dev
npx tauri build
```

Không báo “PASS” nếu chưa chạy lệnh tương ứng trong session hiện tại.

---

## 7. INVARIANTS NGHIỆP VỤ BẮT BUỘC

1. Baseline `46/45/45/#233/105M` không được phá.
2. `pretaxAmount ↔ creditAmount511 = Revenue`.
3. `vatAmount ↔ creditAmount3331 = VAT`.
4. `totalAmount ↔ debitAmount131 = Receivable`.
5. TK3331 và TK131 KHÔNG dùng `pretaxAmount`.
6. Revenue variance và VAT variance KHÔNG netting nhau.
7. `NotChecked != PASS`.
8. UI phải hiển thị rõ `CHƯA ĐỐI CHIẾU` khi chưa đủ kiểm tra.
9. Thiếu evidence → `NeedsReview`, không tự nâng thành PASS.
10. Mọi primary record phải được bảo toàn trong kết quả.
11. Candidate chỉ được consume khi match đã được accepted theo rule.
12. Ambiguous match không được tự chọn một ứng viên tùy tiện.
13. Aggregate match phải có trace đầy đủ các record cấu thành.
14. Không được match hai semantic khác nhau chỉ vì cùng số tiền.
15. Không được coi confidence score là bằng chứng kế toán.
16. Không được silently fallback khi source type/mapping không chắc chắn.
17. Mọi kết luận FAIL/PASS quan trọng phải truy ngược được về file/sheet/row gốc.

---

## 8. DATA MODEL MỤC TIÊU

Không tiếp tục mở rộng bằng cách thêm các cột rời rạc vào một model “universal row” duy nhất nếu semantic khác nhau.

### 8.1. Common provenance

Mọi normalized record nên có:

```text
recordId
sourceId
sourceKind
sourceFile
sheetName
sourceRow
originalValues
normalizationWarnings
```

Nếu transform giá trị:

```text
originalValue
normalizedValue
normalizationRule
```

### 8.2. Customer / Partner

Tối thiểu:

```text
partnerCode
partnerName
partnerTaxId
address
role
status
```

Identity priority:

```text
Tax ID / MST          = strongest external identity
Partner code          = strongest internal identity
Normalized exact name = supporting evidence
Address               = supporting evidence
Fuzzy name            = suggestion only
```

Không auto-match hai pháp nhân chỉ bằng fuzzy name.

### 8.3. Invoice

Tối thiểu:

```text
invoiceNumber
invoiceSeries
invoiceDate
partnerCode
partnerTaxId
partnerName
pretaxAmount
vatAmount
totalAmount
currency
exchangeRate
invoiceStatus
adjustmentType
originalInvoiceRef
replacementInvoiceRef
```

Phải thiết kế được lifecycle:

```text
NORMAL
ADJUSTED
REPLACED
CANCELLED
UNKNOWN
```

Không cộng tất cả invoice rows vào doanh thu nếu lifecycle chưa được hiểu.

### 8.4. Ledger line

Tối thiểu:

```text
postingDate
documentNumber
account
counterAccount
partnerCode
partnerTaxId
partnerName
description
debitAmount
creditAmount
```

Không ép mọi ledger thành `totalAmount`.

### 8.5. Bank transaction / TK112

Tối thiểu:

```text
transactionDate
valueDate
reference
description
bankAccount
amount
direction
signedAmount
```

`direction`:

```text
MONEY_IN
MONEY_OUT
UNKNOWN
```

**P0 RULE:** tiền vào và tiền ra không được match với nhau chỉ vì cùng absolute amount.

---

## 9. SOURCE ROLE VÀ SCENARIO

Không coi tất cả file trong session ngang nhau.

Mỗi source cần role rõ:

```text
PRIMARY
REQUIRED_SECONDARY
OPTIONAL_SECONDARY
REFERENCE_MASTER
```

Ví dụ:

```text
Scenario: INVOICE_VS_511

T7.2026 Thuế.xlsx  = PRIMARY
T7.2026.xlsx       = REQUIRED_SECONDARY
BK.xlsx            = OPTIONAL_SECONDARY
DM Khách hàng.xlsx = REFERENCE_MASTER
```

Mỗi reconciliation scenario phải có config riêng:

```text
scenarioId
primarySourceKinds
secondarySourceKinds
requiredFields
dateTolerance
amountTolerance
aggregatePolicy
identityPolicy
directionPolicy
acceptanceRules
```

Không dùng duy nhất một:

```text
matchingToleranceVnd
dateToleranceDays
enableAggregateMatch
```

cho mọi nghiệp vụ trong tương lai.

---

## 10. MATCHING POLICY

Thứ tự ưu tiên:

```text
1. Validate source + mapping
2. Exact deterministic match
3. Deterministic tolerant match
4. Deterministic aggregate match
5. Ambiguous / insufficient evidence → NeedsReview
```

### Exact match

Ưu tiên:
- invoice/document number;
- amount semantic đúng;
- partner identity;
- date phù hợp scenario.

### Tolerance

Tolerance phải:
- thuộc scenario;
- có lý do nghiệp vụ;
- không tự mở rộng toàn app.

### Aggregate

Hỗ trợ:

```text
1 ↔ 1
1 ↔ N
N ↔ 1
N ↔ M
```

Nhưng phải:
- giới hạn candidate space;
- bảo toàn tổng;
- bảo toàn direction khi áp dụng;
- không consume candidate trước khi accepted;
- cung cấp group evidence.

### Fail-closed

Nếu có hai match gần tương đương:

```text
AMBIGUOUS
```

không được chọn “best guess” rồi PASS.

---

## 11. SMART IMPORT / FILE INTAKE

Mục tiêu UX:

```text
Kéo nhiều file
    ↓
Sniff định dạng thật
    ↓
Detect workbook/sheet/header
    ↓
Detect source kind
    ↓
Suggest mapping
    ↓
Validate
    ↓
User confirm nếu confidence thấp
```

Auto-detect là trợ giúp, không phải quyền được đoán.

Phải phân biệt:
- extension;
- actual file format;
- workbook;
- sheet;
- header row;
- data start row.

Các loại dữ liệu mục tiêu gồm:
- invoice export;
- ledger 511;
- ledger 3331;
- ledger 131;
- ledger 112;
- bank statement;
- customer master;
- sales register;
- custom.

Không được giả định header nằm ở row 1.

Phải xử lý/kiểm thử:
- merged title rows;
- totals/subtotals;
- empty-looking zero cells;
- Excel serial dates;
- leading-zero customer/document codes;
- Vietnamese diacritics;
- tax IDs có branch suffix;
- currency formatting;
- legacy `.xls`;
- `.xlsx`;
- file có extension không khớp format thực.

---

## 12. P0/P1 KIẾN TRÚC CẦN GIẢI QUYẾT TRƯỚC KHI MỞ RỘNG

### P0 — Bank/TK112 direction

Hiện hướng phát triển mới yêu cầu model phải phân biệt:

```text
debitAmount
creditAmount
signedAmount
direction
```

Không release `TK112 ↔ bank statement` nếu engine chưa chứng minh được direction-safe matching.

### P1 — Invoice lifecycle

Bổ sung:
- status;
- adjustment;
- replacement;
- cancellation;
- relationship tới hóa đơn gốc.

Không kết luận doanh thu dựa trên invoice export nếu lifecycle không được xử lý.

### P1 — SourceRole + scenario rules

Bổ sung source role và rule theo scenario trước khi cho phép “kéo toàn bộ file vào và đối chiếu tất cả”.

### P1 — Partner master identity

Bổ sung `partnerCode` và khả năng dùng `DM Khách hàng` như `REFERENCE_MASTER`.

### P1 — Provenance

Mọi result quan trọng phải chỉ ra được:

```text
source file
sheet
row
field
original value
normalized value
rule used
```

Không tạo “black-box result”.

---

## 13. UX NGUYÊN TẮC

Default UI dành cho người kế toán, không dành cho developer.

Nên nói:

```text
Doanh thu chưa được ghi sổ
Hóa đơn chưa thu tiền
Tiền ngân hàng chưa được hạch toán
Khách hàng không khớp mã số thuế
Có nhiều giao dịch có thể khớp
Chưa đủ dữ liệu để kết luận
```

Không bắt người dùng hiểu ngay:

```text
candidate consumption
source role
residual group
confidence threshold
```

Các thông tin kỹ thuật để trong `Chi tiết kỹ thuật / Advanced View`.

Mọi `PASS` phải có ý nghĩa rõ.
Mọi `CHƯA ĐỐI CHIẾU` phải khác `PASS`.
Mọi `NeedsReview` phải giải thích thiếu bằng chứng gì.

---

## 14. BẢO MẬT VÀ QUYỀN RIÊNG TƯ

Mặc định:
- chạy local;
- không upload file kế toán lên cloud;
- không telemetry nếu chưa được phê duyệt;
- không log toàn bộ dữ liệu nhạy cảm ra console/release log;
- không gửi dữ liệu người dùng cho LLM/API ngoài nếu chưa có opt-in rõ ràng.

Khi export CSV/Excel, phải xem xét formula injection (`=`, `+`, `-`, `@`) ở cell xuất ra.

Không sửa file input gốc.

---

## 15. TEST STRATEGY

### 15.1. Test layers

Bắt buộc duy trì:

```text
Unit tests
Parser/normalizer tests
Matching tests
Adversarial tests
Golden dataset tests
IPC contract tests
Frontend contract tests
Packaging smoke tests
```

### 15.2. Baseline golden test

Phải luôn có automated test cho:

```text
46 / 45 / 45 / #233 / 105M
```

### 15.3. Adversarial cases tối thiểu

Cần test:
- cùng số tiền, khác direction;
- cùng số tiền, cùng ngày, khác khách;
- invoice number trùng giữa series khác nhau;
- duplicate row;
- 1↔N;
- N↔1;
- N↔M;
- ambiguous aggregate;
- cancelled invoice;
- replacement invoice;
- adjustment invoice;
- missing MST;
- MST branch suffix;
- partner name variation;
- leading zeros;
- Excel date serial;
- total row bị chèn giữa data;
- sheet sai/không chắc chắn;
- column mapping ambiguous.

### 15.4. False-match budget

Mục tiêu quan trọng nhất:

```text
FALSE_MATCH = 0
```

Nếu không chắc → `NeedsReview`.

---

## 16. DATA CONTRACT / MIGRATION POLICY

Rust ↔ Tauri ↔ TypeScript contract phải version hóa hoặc thay đổi có kiểm soát.

Khi thêm field:
1. ưu tiên additive change;
2. giữ backward compatibility nếu hợp lý;
3. cập nhật Rust model;
4. cập nhật IPC DTO;
5. cập nhật TS contract;
6. cập nhật tests;
7. cập nhật docs;
8. kiểm tra serialization roundtrip.

Không đổi semantic của field cũ mà chỉ giữ nguyên tên.

Ví dụ sai:

```text
totalAmount trước = invoice payable
totalAmount sau = signed bank amount
```

Nếu semantic khác → field/type khác.

---

## 17. DEFINITION OF DONE CHO MỘT FEATURE

Feature chỉ được coi là Done khi:

1. Có mô tả nghiệp vụ.
2. Có input/output contract.
3. Có failure modes.
4. Có tests.
5. Có adversarial tests nếu liên quan matching.
6. Không phá baseline.
7. Rust tests PASS.
8. TS tests PASS.
9. Typecheck PASS.
10. fmt PASS.
11. clippy `-D warnings` PASS.
12. IPC contract đồng bộ.
13. UI không biến `NotChecked` thành PASS.
14. Có provenance/evidence phù hợp.
15. Docs/handoff được cập nhật nếu thay đổi kiến trúc.
16. Release build/smoke test PASS nếu feature đi vào release.

---

## 18. RELEASE GATE

Trước release:

```powershell
git status
cargo test --workspace
npm run test
npm run typecheck
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
npx tauri build
```

Sau đó kiểm tra:
- git tree có sạch hoặc mọi thay đổi đã được giải thích;
- baseline PASS;
- portable EXE smoke PASS;
- không phụ thuộc DLL ngoài ngoài thiết kế;
- version đã bump;
- UI hiển thị version;
- report chứa version/build identifier;
- release artifact có checksum;
- release notes nêu known limitations.

Không phát hành file tên/version cũ sau khi engine đã thay đổi đáng kể.

---

## 19. VERSIONING VÀ TRACEABILITY

App nên hiển thị:

```text
AppKetoan <semver>
Engine <version>
Build date
Git commit
```

Ví dụ:

```text
AppKetoan 0.2.0
Engine 14
Build 2026-08-25
Commit abc1234
```

Report/export cũng nên mang metadata tương tự.

Mục tiêu: khi người dùng báo sai, có thể biết chính xác binary nào tạo kết quả.

---

## 20. TRẠNG THÁI SNAPSHOT TẠI NGÀY BÀN GIAO

**Chỉ là snapshot lịch sử. Agent mới phải xác minh lại.**

Tại 2026-08-25, handoff cũ ghi:

```text
54/54 Rust tests: PASS
18/18 TS tests: PASS
appketoan.exe: ~28.4 MB
No DLL rời cho WebView2Loader patch
Smoke test isolated: SUCCESS
```

Không được dùng các số trên để tuyên bố trạng thái hiện tại nếu chưa chạy lại.

### Uncommitted snapshot được ghi nhận trong handoff cũ

```text
M  Cargo.toml
M  crates/reconciliation-core/src/intake/dedup.rs
M  crates/reconciliation-core/src/matcher/engine.rs
M  crates/reconciliation-core/src/models/data_source.rs
M  crates/reconciliation-core/tests/reconciliation_correctness_regression_test.rs
M  src-tauri/Cargo.toml
M  src-tauri/src/commands.rs
M  src/types/dataContract.test.ts
M  src/utils/money.ts
?? appketoan-standalone-dist/
?? crates/webview2-com-sys/
```

**KHÔNG được blind-commit danh sách trên.**

Agent mới phải:

```powershell
git status
git diff --stat
git diff
git log --oneline -10
```

Sau đó:
1. xác minh từng thay đổi;
2. chạy toàn bộ test gate;
3. thêm `.gitignore` đúng chỗ nếu cần;
4. chỉ commit các file có chủ đích;
5. tạo commit message mô tả thay đổi;
6. ghi lại commit hash vào handoff/session report.

---

## 21. RELEASE FILES SNAPSHOT

Handoff cũ ghi:

```text
D:\appketoan\target\release\appketoan.exe
D:\appketoan\target\release\bundle\nsis\appketoan_0.1.0_x64-setup.exe
D:\appketoan\target\release\bundle\msi\appketoan_0.1.0_x64_en-US.msi
```

Đây là path/version tại snapshot cũ, không phải cam kết rằng artifact hiện tại vẫn nằm đúng path hoặc đúng version.

Agent phải build/xác minh lại trước khi giao file.

---

## 22. LƯU Ý `webview2-com-sys`

Không xóa hoặc sửa:

```text
crates/webview2-com-sys/
```

nếu chưa hiểu rõ toàn bộ lý do tồn tại.

Patch hiện được mô tả là:
- static link `WebView2Loader` cho GNU toolchain;
- `msvc_compat.s` cung cấp GNU stubs cho MSVC CRT symbols;
- root `Cargo.toml` có `[patch.crates-io]` trỏ vào bản vendored.

Bất kỳ thay đổi nào ở đây phải:
1. build release;
2. chạy portable EXE trên môi trường sạch;
3. xác minh không phát sinh DLL dependency ngoài ý muốn;
4. ghi ADR hoặc commit note giải thích.

---

## 23. QUY TRÌNH AGENT MỚI TIẾP QUẢN

Ngay khi bắt đầu session mới:

### Bước 1 — Không sửa code

Đọc:
- `AGENT_HANDOFF.md`;
- `docs/`;
- ADR liên quan;
- test names;
- git history gần nhất.

### Bước 2 — Xác minh repo

```powershell
cd D:\appketoan
git status
git diff --stat
git log --oneline -10
```

### Bước 3 — Xác minh baseline

Chạy:

```powershell
cargo test --workspace
npm run test
npm run typecheck
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### Bước 4 — Xác định task

Mỗi task phải ghi:
- vấn đề;
- expected behavior;
- actual behavior;
- affected workflow;
- risk;
- files dự kiến sửa;
- tests phải thêm.

### Bước 5 — Sửa nhỏ, atomic

Không rewrite rộng nếu chưa cần.

Ưu tiên:
- fix root cause;
- preserve compatibility;
- test trước/sau;
- không “chắp vá” bằng special-case dataset nếu rule không có ý nghĩa nghiệp vụ.

### Bước 6 — Báo cáo

Cuối session agent phải trả:

```text
STATUS
- PASS / PARTIAL / BLOCKED

CHANGES
- ...

TESTS
- Rust:
- TS:
- typecheck:
- fmt:
- clippy:
- build:
- smoke:

BASELINE
- 46/45/45/#233/105M: PASS/FAIL

KNOWN RISKS
- ...

GIT
- working tree:
- commit:
- branch:

NEXT
- ...
```

---

## 24. THỨ TỰ ƯU TIÊN TƯƠNG LAI

Không khóa cứng timeline; ưu tiên theo dependency.

### Priority 0

1. Bảo toàn baseline hiện tại.
2. Xác minh/ổn định git state.
3. Không false-match.
4. Không phá money semantics.
5. Version/build traceability.

### Priority 1

1. Bank/TK112 signed direction model.
2. Invoice lifecycle.
3. SourceRole.
4. Scenario-specific reconciliation rules.
5. `partnerCode` + customer master identity.
6. Provenance hoàn chỉnh.

### Priority 2

1. Smart Import mạnh hơn.
2. Bảng kê ↔ hóa đơn ↔ ledger.
3. Month-end control dashboard.
4. Robust Excel/XLS intake.
5. Report evidence drill-down.

### Priority 3

1. AI explanation layer.
2. Rule recommendation.
3. Template learning từ mapping đã xác nhận.

AI không được đi trước deterministic core.

---

## 25. NGUYÊN TẮC CUỐI

Nếu agent gặp lựa chọn giữa:

```text
A. Match nhiều hơn nhưng có nguy cơ match sai
B. Match ít hơn và đưa phần không chắc sang NeedsReview
```

**chọn B.**

Nếu gặp lựa chọn giữa:

```text
A. UI trông đẹp nhưng che mất uncertainty
B. UI đơn giản nhưng nói rõ CHƯA ĐỦ BẰNG CHỨNG
```

**chọn B.**

Nếu gặp lựa chọn giữa:

```text
A. Sửa nhanh bằng special-case
B. Xác định semantic đúng rồi sửa root cause
```

**chọn B.**

Mục tiêu cao nhất của AppKetoan không phải “match được nhiều”.

Mục tiêu là:

> **Không tạo ra sự tự tin giả trong một kết luận kế toán.**

---

## 26. V15 REAL-DATASET GATE (2026-08-25)

The V15 implementation must not start until all required real workbooks are available locally under `.local-testdata/`. This directory is intentionally ignored and must never be committed. If the directory or any required workbook is missing, record the blocked state and do not infer source structure from filenames or historical assumptions.
