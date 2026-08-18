# UX Specifications: Workflows & Screen Architecture

## 1. Core UX Philosophy
The interface is designed for Vietnamese accounting professionals who need speed, clarity, and zero cognitive overload.
- **Zero Technical Jargon**: No database terminology, no regex configurations required from accountants.
- **Guided 3-Step Wizard**:
  1. **Select Scenario / Profile (Chọn Kịch bản đối chiếu)**
  2. **Ingest Files & Confirm Mapping (Tải file & Xác nhận cột)**
  3. **View Results & Audit Discrepancies (Xem kết quả & Xử lý sai lệch)**

---

## 2. Step-by-Step Workflow

### Step 1: Select Reconciliation Scenario (Chọn Kịch bản)
User selects from standard pre-configured cards:
- **Kịch bản 1: Đối chiếu Doanh thu & Thuế đầu ra (3 nguồn)**
  - Hóa đơn điện tử (HĐĐT) $\leftrightarrow$ Sổ chi tiết TK 511 $\leftrightarrow$ Bảng kê thuế TK 3331.
- **Kịch bản 2: Đối chiếu Doanh thu, Thuế & Công nợ (4 nguồn)**
  - HĐĐT $\leftrightarrow$ Sổ TK 511 $\leftrightarrow$ Sổ TK 3331 $\leftrightarrow$ Sổ TK 131.
- **Kịch bản 3: Đối chiếu Tiền gửi & Sao kê Ngân hàng (2 nguồn)**
  - Sổ tiền gửi ngân hàng (TK 112) $\leftrightarrow$ File sao kê ngân hàng (VCB, BIDV, TCB, ACB, MB).
- **Kịch bản 4: Đối chiếu Thu tiền & Công nợ (3 nguồn)**
  - Sổ công nợ TK 131 $\leftrightarrow$ Phiếu thu tiền mặt $\leftrightarrow$ Sao kê ngân hàng.
- **Kịch bản 5: Kịch bản tùy biến (Custom Multi-Source)**
  - Tự do thêm N file Excel và cấu hình rule so khớp.

---

### Step 2: Drag & Drop Ingestion & Intelligent Auto-Mapping
1. **Dropzone per Source**: Each required source displays a clean dropzone.
2. **Sheet Selector**: If a workbook has multiple sheets, a dropdown automatically lists available sheets (e.g. `HĐ Đã Phát Hành`, `Bảng Kê Hóa Đơn`).
3. **Auto-Detection Engine**: The system reads the first 10 rows and automatically identifies:
   - Header row index (bỏ qua dòng tiêu đề báo cáo, tên công ty ở đầu file).
   - Column bindings based on Vietnamese accounting keywords:
     - `Số HĐ` / `Số hóa đơn` / `Invoice No` $\rightarrow$ `docNo`
     - `Ký hiệu` / `Mẫu số` / `Series` $\rightarrow$ `series`
     - `Mã số thuế` / `MST` / `Tax Code` $\rightarrow$ `partnerTaxId`
     - `Tên khách hàng` / `Tên đơn vị` / `Tên người mua` $\rightarrow$ `partnerName`
     - `Tiền chưa thuế` / `Doanh thu` / `Doanh số` $\rightarrow$ `pretaxAmount`
     - `Tiền thuế` / `Thuế GTGT` $\rightarrow$ `vatAmount`
     - `Tổng tiền` / `Tổng cộng` / `Thành tiền` $\rightarrow$ `totalAmount`
     - `Ngày HĐ` / `Ngày chứng từ` / `Ngày giao dịch` $\rightarrow$ `date`
     - `TK Nợ` $\rightarrow$ `debitAccount`, `TK Có` $\rightarrow$ `creditAccount`
     - `Số chứng từ` / `Số CT` / `Số phiếu` $\rightarrow$ `voucherNo`
4. **Mapping Confidence Indicator**:
   - If confidence $\ge 90\%$, auto-check with green badge.
   - If confidence $< 90\%$, open compact inline mapping modal for user to select the matching column.

---

### Step 3: Instant Reconciliation Dashboard & Results Grid

#### A. KPI Metrics Bar
- **Tổng số chứng từ (Total Records)** across all sources.
- **Khớp hoàn toàn (Matched Exact)** (Green badge).
- **Khớp có chênh lệch / Cảnh báo (Matched with Warning)** (Yellow badge).
- **Sai lệch số tiền / Thuế (Amount / VAT Mismatches)** (Red badge).
- **Thiếu chứng từ (Missing Records)** (Red/Orange badge).
- **Nghi ngờ trùng lặp (Duplicate Suspects)** (Purple badge).
- **Cần kiểm tra lại (Ambiguous Matches)** (Orange badge).
- **Tổng chênh lệch tài chính (Net Financial Variance)** in VND.

#### B. Filter & Search Toolbar
- **Tabs**: Tất cả | Khớp 100% | Lệch tiền/thuế | Thiếu bên HĐ | Thiếu bên Sổ | Trùng lặp.
- **Search input**: Quick filter by Invoice No, Tax ID, Partner Name, Voucher No.
- **Amount range filter** and **Date range filter**.

#### C. Detail Comparison Table & Side-by-Side Diff Inspector
- Expanding any row reveals a side-by-side field comparison table:
  - Highlights differing cells with light-red background.
  - Explains the exact discrepancy reason (e.g. `Thuế suất HĐĐT là 10% (2.000.000đ) nhưng Sổ cái hạch toán 8% (1.600.000đ)`).

#### D. Export Action
- **Xuất Báo Cáo Excel (Export XLSX)**: Generates a multi-sheet audit workbook with formatted summary, matched pairs, and discrepancy lists with formulas.
