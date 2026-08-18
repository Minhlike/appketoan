import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App Bootstrap Component", () => {
  it("renders Accounting Reconciliation bootstrap title and subtitle", () => {
    render(<App />);
    expect(screen.getByText("Accounting Reconciliation")).toBeDefined();
    expect(screen.getByText("Bootstrap Ready")).toBeDefined();
    expect(screen.getByText("System Status: Online")).toBeDefined();
  });
});
