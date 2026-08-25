import type { DataSource, ExcelFileMetadata } from "./dataContract";

export interface IngestedSourceItem {
  id: string;
  source: DataSource;
  fileMetadata: ExcelFileMetadata;
  rawBytes?: Uint8Array;
  rawSha256?: string;
}

export type AuditOperationState =
  | "IDLE"
  | "READY"
  | "RUNNING"
  | "CANCELLING"
  | "COMPLETED"
  | "PARTIAL"
  | "ERROR";
