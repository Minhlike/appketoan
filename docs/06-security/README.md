# Security Baseline & Confidentiality

## Principles
1. **Zero Data Exfiltration**: Dữ liệu kế toán tuyệt đối không bao giờ được gửi qua Internet hay cloud service.
2. **Offline-First**: Mọi nghiệp vụ đọc file, tính toán, đối chiếu, xuất báo cáo đều chạy 100% offline trên máy local.
3. **No Telemetry / No Crash Reports**: Tắt toàn bộ telemetry, analytics, và tracking.
4. **Git Hygiene**: Cấm commit file kế toán thực tế (`.xlsx`, `.xls`, `.csv`, sao kê, hóa đơn). Chỉ cho phép file giả lập trong `fixtures/synthetic/`.
