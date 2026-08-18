# Data Contract: Reconciliation Results & Profiles

## 1. Reconciliation Profile & Matching Rules

```typescript
export type MatchKeyType =
  | "doc_no"
  | "series_and_doc_no"
  | "tax_id_and_amount"
  | "tax_id_and_doc_no"
  | "custom_keys";

export interface MatchingRule {
  id: string;
  name: string;
  primaryKeys: MatchKeyType[];
  allowDateVarianceDays: number;
  allowAmountToleranceVnd: number;
  enableAggregateMatch: boolean;
  aggregateGroupingKeys?: string[];
}

export interface ReconciliationProfile {
  id: string;
  name: string;
  sourceIds: string[];
  rules: MatchingRule[];
}
```

---

## 2. Reconciliation Output Schema

```typescript
export type MatchStatus =
  | "MATCHED_EXACT"
  | "MATCHED_WITH_TOLERANCE"
  | "MATCHED_AGGREGATE"
  | "MISMATCH_AMOUNT"
  | "MISMATCH_METADATA"
  | "UNMATCHED_MISSING_IN_TARGET"
  | "UNMATCHED_MISSING_IN_SOURCE"
  | "DUPLICATE_SUSPECT"
  | "AMBIGUOUS_MATCH";

export interface FieldDiscrepancy {
  fieldName: string;
  sourceValue?: string;
  targetValue?: string;
  amountDiff?: number;
  message: string;
}

export interface MatchGroup {
  id: string;
  status: MatchStatus;
  primarySourceRecordIds: string[];
  targetSourceRecordIds: string[];
  discrepancies: FieldDiscrepancy[];
  totalSourceAmount: number;
  totalTargetAmount: number;
  amountVariance: number;
}

export interface ReconciliationSummary {
  totalSourceRecords: number;
  totalTargetRecords: number;
  exactMatchesCount: number;
  toleranceMatchesCount: number;
  aggregateMatchesCount: number;
  mismatchesCount: number;
  missingInTargetCount: number;
  missingInSourceCount: number;
  duplicatesCount: number;
  ambiguousCount: number;
  netFinancialVariance: number;
}

export interface ReconciliationResult {
  sessionId: string;
  executedAt: string;
  profileId: string;
  summary: ReconciliationSummary;
  groups: MatchGroup[];
}
```
