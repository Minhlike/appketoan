# Accounting Rules: Built-in Reconciliation Profiles

## 1. Profile 1: Revenue & Output VAT 3-Way Reconciliation
**Mục tiêu**: Kiểm tra tính đầy đủ và chính xác giữa Hóa đơn điện tử xuất ra, Doanh thu hạch toán (TK 511) và Thuế GTGT đầu ra (TK 3331).

### Sources Involved
- `Source A`: Bảng kê Hóa đơn điện tử đầu ra (Tổng cục Thuế / Nhà cung cấp HĐĐT).
- `Source B`: Sổ chi tiết Doanh thu (TK 511).
- `Source C`: Sổ chi tiết Thuế GTGT phải nộp (TK 33311).

### Matching Rules & Verification Invariants
1. **Primary Key**: `Series` + `DocNo` + `PartnerTaxId`.
2. **Financial Invariant**:
   $$\text{TotalAmount}_A = \text{PretaxAmount}_B + \text{VatAmount}_C$$
   $$\text{PretaxAmount}_A = \text{PretaxAmount}_B$$
   $$\text{VatAmount}_A = \text{VatAmount}_C$$
3. **Tolerance**: Allowable amount variance $\le 10$ VND (rounding difference). Allowable date variance $\le 3$ days.

---

## 2. Profile 2: Revenue, Output VAT & Receivables 4-Way Reconciliation
**Mục tiêu**: Đối chiếu toàn diện chu trình bán hàng từ Hóa đơn $\rightarrow$ Doanh thu $\rightarrow$ Thuế $\rightarrow$ Công nợ khách hàng.

### Sources Involved
- `Source A`: Hóa đơn điện tử đầu ra.
- `Source B`: Sổ chi tiết TK 511.
- `Source C`: Sổ chi tiết TK 33311.
- `Source D`: Sổ chi tiết TK 131 (Phát sinh Nợ).

### Matching Rules & Verification Invariants
1. **Primary Key**: `DocNo` + `PartnerTaxId`.
2. **Financial Invariant**:
   $$\text{DebitAmount}_D = \text{TotalAmount}_A = \text{CreditAmount}_B + \text{CreditAmount}_C$$

---

## 3. Profile 3: Bank Account Reconciliation (Đối chiếu Tiền gửi Ngân hàng)
**Mục tiêu**: Đối chiếu Sổ tiền gửi ngân hàng (TK 112) của doanh nghiệp với File sao kê thực tế từ ngân hàng thương mại.

### Sources Involved
- `Source A`: Sổ tiền gửi ngân hàng (TK 112) - Chi tiết theo số tài khoản.
- `Source B`: File sao kê ngân hàng (Excel export từ Internet Banking).

### Matching Rules & Verification Invariants
1. **Primary Key Matching**:
   - Level 1: Số tham chiếu / Reference Code / Mã GD ngân hàng trong diễn giải.
   - Level 2: `TransactionDate` (sai lệch $\le 2$ ngày) + `Amount` + `PartnerAccount/Name`.
2. **Direction Invariant**:
   $$\text{Sổ 112 (Nợ - Tiền vào)} \leftrightarrow \text{Sao kê (Có - Credit/Ghi có)}$$
   $$\text{Sổ 112 (Có - Tiền ra)} \leftrightarrow \text{Sao kê (Nợ - Debit/Ghi nợ)}$$
3. **Tolerance**: Amount $\Delta = 0$. Date $\Delta t \le 3$ days.

---

## 4. Profile 4: Collection & Receivables Settlement (Thu tiền & Giảm trừ công nợ)
**Mục tiêu**: Đối chiếu các khoản thanh toán của khách hàng (Phiếu thu tiền mặt hoặc Báo có ngân hàng) với việc hạch toán giảm công nợ TK 131.

### Sources Involved
- `Source A`: Sổ công nợ TK 131 (Phát sinh Có - Giảm công nợ).
- `Source B`: Sổ quỹ tiền mặt (Phiếu thu 111) & Sổ tiền gửi (Báo có 112).
- `Source C`: Sao kê ngân hàng.

### Matching Rules & Cardinality
- Supports $1 \leftrightarrow N$ (Một lần chuyển khoản ngân hàng thanh toán cho nhiều hóa đơn/chứng từ công nợ).
- Grouping key: `PartnerTaxId` or `PartnerCode`.

---

## 5. Profile 5: Multi-Branch to HQ Consolidation (Hợp nhất Chi nhánh $\leftrightarrow$ Tổng công ty)
**Mục tiêu**: Đối chiếu số liệu giữa nhiều sổ chi nhánh (Branch 1, Branch 2, Branch N) với sổ kế toán tổng hợp tại Tổng công ty.

### Sources Involved
- `Source A1, A2, ..., An`: Sổ chi tiết các chi nhánh / đơn vị thành viên.
- `Source B`: Sổ tổng hợp toàn công ty.

### Matching Rules
- Aggregate Match: $\sum_{k=1}^n \text{Branch}_k = \text{HQ Total}$ theo từng tài khoản đối ứng và mã khách hàng.
