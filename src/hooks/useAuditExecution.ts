import { useEffect, useMemo, useRef, useState } from "react";

import {
  cancelAuditWorkspace,
  resetAuditWorkspace,
  runAuditWorkspace,
  safeUserError,
} from "../services/api";
import type {
  AccountingPeriod,
  AuditWorkspaceReport,
  MoneyValue,
} from "../types/dataContract";
import type { AuditOperationState, IngestedSourceItem } from "../types/auditWorkspace";

const sessionId = () =>
  `audit_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

export function useAuditExecution(
  sources: IngestedSourceItem[],
  toleranceVnd: MoneyValue,
  dateToleranceDays: number
) {
  const sessionIdRef = useRef(sessionId());
  const [accountingPeriod, setAccountingPeriod] = useState<AccountingPeriod>({
    startDate: "",
    endDate: "",
  });
  const [report, setReport] = useState<AuditWorkspaceReport | null>(null);
  const [operationState, setOperationState] = useState<AuditOperationState>("IDLE");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const isBusy = operationState === "RUNNING" || operationState === "CANCELLING";
  const canRun = useMemo(
    () =>
      sources.length > 0 &&
      Boolean(accountingPeriod.startDate) &&
      Boolean(accountingPeriod.endDate) &&
      accountingPeriod.startDate <= accountingPeriod.endDate &&
      !isBusy,
    [accountingPeriod, isBusy, sources.length]
  );

  useEffect(() => {
    setReport(null);
    setErrorMessage(null);
    setOperationState(sources.length > 0 ? "READY" : "IDLE");
  }, [accountingPeriod.startDate, accountingPeriod.endDate, sources.length]);

  const invalidate = (hasSources = sources.length > 0) => {
    setReport(null);
    setErrorMessage(null);
    setOperationState(hasSources ? "READY" : "IDLE");
  };

  const run = async () => {
    if (!canRun) {
      setErrorMessage(
        sources.length === 0
          ? "Vui lòng nạp ít nhất một nguồn vào bộ hồ sơ kế toán."
          : "Vui lòng chọn kỳ kế toán hợp lệ."
      );
      return;
    }
    setOperationState("RUNNING");
    setErrorMessage(null);
    const invocationStarted = performance.now();
    try {
      const fileBytesMap: Record<string, number[]> = {};
      for (const source of sources) {
        if (source.rawBytes) fileBytesMap[source.id] = Array.from(source.rawBytes);
      }
      const next = await runAuditWorkspace(
        sessionIdRef.current,
        accountingPeriod,
        sources.map((item) => item.source),
        String(toleranceVnd),
        dateToleranceDays,
        Object.keys(fileBytesMap).length > 0 ? fileBytesMap : undefined
      );
      const backendReturnedAt = performance.now();
      next.metrics.ipcInclusiveMs = Math.round(backendReturnedAt - invocationStarted);
      next.metrics.ipcRoundTripOverheadMs = Math.max(
        0,
        next.metrics.ipcInclusiveMs - next.metrics.stages.totalBackendMs
      );
      setReport(next);
      requestAnimationFrame(() => {
        const frontendRenderMs = Math.round(performance.now() - backendReturnedAt);
        setReport((current) =>
          current?.sessionId === next.sessionId
            ? {
                ...current,
                metrics: { ...current.metrics, frontendRenderMs },
              }
            : current
        );
      });
      setOperationState(
        next.runStatus === "PARTIAL"
          ? "PARTIAL"
          : next.runStatus === "FAILED" || next.runStatus === "CANCELLED"
            ? "ERROR"
            : "COMPLETED"
      );
    } catch (error: unknown) {
      setErrorMessage(safeUserError(error));
      setOperationState("ERROR");
    }
  };

  const cancel = async () => {
    if (operationState !== "RUNNING") return;
    setOperationState("CANCELLING");
    try {
      const accepted = await cancelAuditWorkspace(sessionIdRef.current);
      if (!accepted) {
        setErrorMessage("Không tìm thấy tiến trình đang chạy để dừng.");
        setOperationState("ERROR");
      }
    } catch (error: unknown) {
      setErrorMessage(safeUserError(error));
      setOperationState("ERROR");
    }
  };

  const reset = () => {
    void resetAuditWorkspace(sessionIdRef.current).catch(() => undefined);
    sessionIdRef.current = sessionId();
    setReport(null);
    setErrorMessage(null);
    setOperationState("IDLE");
  };

  const resetPreparedSession = () => {
    void resetAuditWorkspace(sessionIdRef.current).catch(() => undefined);
    sessionIdRef.current = sessionId();
    setReport(null);
    setErrorMessage(null);
    setOperationState(sources.length > 1 ? "READY" : "IDLE");
  };

  return {
    accountingPeriod,
    setAccountingPeriod,
    report,
    setReport,
    operationState,
    errorMessage,
    setErrorMessage,
    isBusy,
    canRun,
    run,
    cancel,
    reset,
    resetPreparedSession,
    invalidate,
  };
}
