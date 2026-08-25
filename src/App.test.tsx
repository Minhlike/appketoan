import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import App from "./App";

describe("AppKetoan UI End-to-End Workflow", () => {
  it("renders main header, offline security badge, and scenario selector", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { name: /Đối chiếu số liệu kế toán/i })
    ).toBeDefined();
    expect(screen.getByText(/Offline Bảo mật/i)).toBeDefined();
    expect(screen.getByText(/Kịch bản đối chiếu:/i)).toBeDefined();
    expect(screen.getByText(/📁 Nạp dữ liệu mẫu/i)).toBeDefined();
  });

  it("loads demo data and enables the 'CHẠY ĐỐI CHIẾU' button", () => {
    render(<App />);
    const demoBtn = screen.getByText(/📁 Nạp dữ liệu mẫu/i);
    fireEvent.click(demoBtn);

    // Should display the 2 demo sources
    expect(screen.getByText(/Nguồn #1/i)).toBeDefined();
    expect(screen.getByText(/Nguồn #2/i)).toBeDefined();
    expect(screen.getByText(/2 nguồn/i)).toBeDefined();

    const runBtn = screen.getByRole("button", { name: /▶ CHẠY ĐỐI CHIẾU/i });
    expect(runBtn.hasAttribute("disabled")).toBe(false);
  });

  it("executes reconciliation on demo datasets and renders Dashboard KPIs & Result Table", async () => {
    render(<App />);
    // 1. Click load demo data
    const demoBtn = screen.getByText(/📁 Nạp dữ liệu mẫu/i);
    fireEvent.click(demoBtn);

    // 2. Click run reconciliation
    const runBtn = screen.getByRole("button", { name: /▶ CHẠY ĐỐI CHIẾU/i });
    fireEvent.click(runBtn);

    // 3. Wait for results to render
    await waitFor(() => {
      expect(screen.getByText(/Kết quả đối chiếu tổng quan/i)).toBeDefined();
    });

    // Check KPI metric cards
    expect(screen.getByText(/Khớp hoàn toàn \(100%\)/i)).toBeDefined();
    expect(screen.getByText(/Sai lệch tiền \/ Thuế/i)).toBeDefined();
    expect(screen.getAllByText(/Thiếu bên đối chiếu/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Nghi ngờ trùng lặp/i).length).toBeGreaterThan(0);

    // Check action buttons
    expect(screen.getByText(/📊 Xuất Báo Cáo Excel/i)).toBeDefined();

    // Check table headers
    expect(screen.getByText(/Doanh thu \(theo nguồn\)/i)).toBeDefined();
    expect(screen.getByText(/Thuế GTGT \(theo nguồn\)/i)).toBeDefined();
    expect(screen.getByText(/Chi tiết & Lý do sai lệch/i)).toBeDefined();
  });

  it("opens Detail Inspector modal when clicking 'Xem' on a row", async () => {
    render(<App />);
    fireEvent.click(screen.getByText(/📁 Nạp dữ liệu mẫu/i));
    fireEvent.click(screen.getByRole("button", { name: /▶ CHẠY ĐỐI CHIẾU/i }));

    await waitFor(() => {
      expect(screen.getByText(/Kết quả đối chiếu tổng quan/i)).toBeDefined();
    });

    const inspectButtons = screen.getAllByText(/🔍 Xem/i);
    expect(inspectButtons.length).toBeGreaterThan(0);
    fireEvent.click(inspectButtons[0]);

    // Modal should open
    expect(
      screen.getByText(/Kiểm tra chi tiết đối chiếu theo bản chất kế toán/i)
    ).toBeDefined();
    expect(screen.getByText(/Số chứng từ:/i)).toBeDefined();
  });
});
