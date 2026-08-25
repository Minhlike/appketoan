import type { DataSourceKind, SourceRole } from "../types/dataContract";
import type { IngestedSourceItem } from "../types/auditWorkspace";
import { FileIngestionDropzone } from "./FileIngestionDropzone";

interface Props {
  sources: IngestedSourceItem[];
  disabled: boolean;
  advancedMode: boolean;
  onAddSource: (item: IngestedSourceItem) => void;
  onRemoveSource: (id: string) => void;
  onUpdateSheet: (id: string, sheet: string) => void;
  onUpdateKind: (id: string, kind: DataSourceKind) => void;
  onUpdateRole: (id: string, role: SourceRole) => void;
  onOpenMapping: (item: IngestedSourceItem) => void;
}

export function SourceIntakePanel(props: Props) {
  return (
    <FileIngestionDropzone
      sources={props.sources}
      onAddSource={props.onAddSource}
      onRemoveSource={props.onRemoveSource}
      onUpdateSheet={props.onUpdateSheet}
      onUpdateKind={props.onUpdateKind}
      onUpdateRole={props.onUpdateRole}
      onOpenMapping={props.onOpenMapping}
      disabled={props.disabled}
      advancedMode={props.advancedMode}
    />
  );
}
