# AGENTS.md — AI Agent Operating Guidelines & Rules

## 1. MANDATORY START-OF-SESSION PROTOCOL
When beginning any work session, all AI Agents MUST read the following memory files in order:
1. [AGENTS.md](file:///D:/appketoan/AGENTS.md)
2. [.agent/PROJECT_STATE.md](file:///D:/appketoan/.agent/PROJECT_STATE.md)
3. [.agent/CURRENT_TASK.md](file:///D:/appketoan/.agent/CURRENT_TASK.md)
4. [.agent/DECISIONS.md](file:///D:/appketoan/.agent/DECISIONS.md)
5. [.agent/HANDOFF.md](file:///D:/appketoan/.agent/HANDOFF.md)
6. [.agent/TEST_STATUS.md](file:///D:/appketoan/.agent/TEST_STATUS.md)

Only after reviewing these files and understanding the current state and boundaries may any code changes be performed.

---

## 2. MANDATORY END-OF-SESSION PROTOCOL
Before concluding a work session or declaring a task complete, all AI Agents MUST update:
- [.agent/PROJECT_STATE.md](file:///D:/appketoan/.agent/PROJECT_STATE.md)
- [.agent/CURRENT_TASK.md](file:///D:/appketoan/.agent/CURRENT_TASK.md)
- [.agent/HANDOFF.md](file:///D:/appketoan/.agent/HANDOFF.md)
- [.agent/TEST_STATUS.md](file:///D:/appketoan/.agent/TEST_STATUS.md)

If an architectural decision was made:
- Update [.agent/DECISIONS.md](file:///D:/appketoan/.agent/DECISIONS.md) and create an ADR in `docs/04-architecture/adr/`.

If an important lesson or trap was encountered:
- Update [.agent/LESSONS.md](file:///D:/appketoan/.agent/LESSONS.md).

**NEVER mark a session complete without updating agent memory.**

---

## 3. STRICT DATA PRIVACY & ZERO EXFILTRATION
This application processes confidential accounting records.
- **NEVER** upload, transmit, or proxy data to external APIs, cloud services, telemetry servers, or third-party AI services.
- **NEVER** commit real accounting data (`.xlsx`, `.xls`, `.csv`, statements, invoices, company names, tax IDs).
- **NEVER** write real financial numbers or sensitive company identifiers into `.agent/` memory files.
- **ONLY** synthetic/mock data is permitted in `fixtures/synthetic/`.

---

## 4. ARCHITECTURAL BOUNDARIES
- **Multi-Source Engine**: The system is a generic multi-source reconciliation platform. Never hard-code assumptions that only 2 files (`leftFile` / `rightFile`) exist.
- **Local Rust Core**: Business logic, workbook parsing, normalization, validation, and reconciliation algorithms live exclusively in local Rust modules.
- **Minimal Frontend**: React/TypeScript frontend is strictly responsible for presentation, user interaction, filtering, sorting, and reporting views.
- **Simplicity First**: Write minimal code that solves the problem. No speculative abstractions, no unused frameworks.
