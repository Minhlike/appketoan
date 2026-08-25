import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FileIngestionDropzone } from "./FileIngestionDropzone";

describe("FileIngestionDropzone", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("ignores and reports Office lock files without creating a logical source", async () => {
    const onAddSource = vi.fn();
    render(
      <FileIngestionDropzone
        sources={[]}
        onAddSource={onAddSource}
        onRemoveSource={() => undefined}
        onUpdateSheet={() => undefined}
        onOpenMapping={() => undefined}
      />
    );
    const dropzone = screen.getByText(/Kéo thả các file Excel vào đây/).closest(".dropzone-container");
    const lockFile = new File([new Uint8Array([1, 2, 3])], "~$ledger.xlsx");
    fireEvent.drop(dropzone as Element, { dataTransfer: { files: [lockFile] } });
    await waitFor(() => {
      expect(screen.getByText(/Đã bỏ qua tệp tạm/)).toBeDefined();
    });
    expect(onAddSource).not.toHaveBeenCalled();
  });

  it("reports exact byte duplicates within the same multi-file intake", async () => {
    vi.stubGlobal("crypto", {
      subtle: {
        digest: vi.fn(async () => new Uint8Array(32).fill(7).buffer),
      },
    });
    const onAddSource = vi.fn();
    render(
      <FileIngestionDropzone
        sources={[]}
        onAddSource={onAddSource}
        onRemoveSource={() => undefined}
        onUpdateSheet={() => undefined}
        onOpenMapping={() => undefined}
      />
    );
    const dropzone = screen.getByText(/Kéo thả các file Excel vào đây/).closest(".dropzone-container");
    const first = new File([new Uint8Array([1, 2, 3])], "first.xlsx");
    const duplicate = new File([new Uint8Array([1, 2, 3])], "copy.xlsx");
    Object.defineProperty(first, "arrayBuffer", {
      value: async () => new Uint8Array([1, 2, 3]).buffer,
    });
    Object.defineProperty(duplicate, "arrayBuffer", {
      value: async () => new Uint8Array([1, 2, 3]).buffer,
    });
    fireEvent.drop(dropzone as Element, {
      dataTransfer: { files: [first, duplicate] },
    });
    await waitFor(() => {
      expect(screen.getByText(/trùng hoàn toàn với nguồn đã nạp/)).toBeDefined();
    });
    expect(onAddSource).toHaveBeenCalledTimes(1);
  });
});
