# Accounting Standards: Vietnamese Practice (VAS, Circular 200/133, Decree 123)

## 1. Accounting Account System (Hệ thống tài khoản kế toán)
In accordance with Circular 200/2014/TT-BTC and Circular 133/2016/TT-BTC:

- **TK 511 (Doanh thu bán hàng và cung cấp dịch vụ)**:
  - `5111`: Doanh thu bán hàng hóa.
  - `5112`: Doanh thu bán thành phẩm.
  - `5113`: Doanh thu cung cấp dịch vụ.
  - `5118`: Doanh thu khác.
- **TK 3331 (Thuế giá trị gia tăng phải nộp)**:
  - `33311`: Thuế GTGT đầu ra.
  - `33312`: Thuế GTGT hàng nhập khẩu.
- **TK 133 (Thuế GTGT được khấu trừ)**:
  - `1331`: Thuế GTGT được khấu trừ của hàng hóa, dịch vụ.
  - `1332`: Thuế GTGT được khấu trừ của tài sản cố định.
- **TK 131 (Phải thu của khách hàng)**:
  - Chi tiết theo từng đối tượng khách hàng (Mã khách hàng / MST).
- **TK 111 (Tiền mặt), TK 112 (Tiền gửi ngân hàng)**:
  - Chi tiết theo tài khoản ngân hàng và loại tiền tệ (VND / Ngoại tệ).

---

## 2. Electronic Invoices Regulations (Decree 123/2020/NĐ-CP & Circular 78/2021/TT-BTC)

### 2.1 Mandatory Identifier Tuple
An electronic invoice under Decree 123 is uniquely identified by:
1. **Invoice Type / Template Code (Mẫu số hóa đơn)**: e.g. `1` (HĐGTGT), `2` (HĐ Bán hàng).
2. **Invoice Series / Symbol (Ký hiệu hóa đơn)**: e.g. `1C24TAA`, `1K24TMM`.
3. **Invoice Number (Số hóa đơn)**: Up to 8 digits, padded with leading zeros (e.g. `00000123` or `123`).
4. **Seller Tax ID (Mã số thuế bên bán)**: 10 or 14 digits (e.g. `0101234567` or `0101234567-001`).

### 2.2 Normalization Rules for Identifiers
- **Invoice Number Normalization**: Stripping leading zeros and whitespace so that `00001234`, `001234`, `1234`, and `1234.0` evaluate to the same canonical integer/string key `1234`.
- **Tax ID Normalization**: Stripping whitespace, non-numeric dashes unless 14-digit branch code format (`XXXXXXXXXX-XXX`).
- **Date Format Handling**: Handling multiple Excel cell formats (`DD/MM/YYYY`, `YYYY-MM-DD`, Excel numeric serial dates like `45300`).

### 2.3 VAT Rates (Thuế suất GTGT)
- Standard Rates: `0%`, `5%`, `10%`.
- Temporary VAT Reduction (Decrees on 2% VAT reduction): `8%`.
- Non-taxable: `KCT` (Không chịu thuế), `KKKNT` (Không kê khai nộp thuế).
- Formula: $\text{Total Amount} = \text{Pre-tax Amount} + \text{VAT Amount}$.
