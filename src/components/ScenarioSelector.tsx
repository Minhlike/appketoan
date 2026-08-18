import React from "react";
import type { PreconfiguredScenario } from "../types/dataContract";
import { PRECONFIGURED_SCENARIOS } from "../services/api";

interface ScenarioSelectorProps {
  selectedScenarioId: string;
  onSelectScenario: (scenario: PreconfiguredScenario) => void;
  disabled?: boolean;
}

export const ScenarioSelector: React.FC<ScenarioSelectorProps> = ({
  selectedScenarioId,
  onSelectScenario,
  disabled = false,
}) => {
  const currentScenario =
    PRECONFIGURED_SCENARIOS.find((s) => s.id === selectedScenarioId) ||
    PRECONFIGURED_SCENARIOS[0];

  return (
    <section className="scenario-section">
      <div className="section-header-inline">
        <label htmlFor="scenario-select" className="section-label">
          Kịch bản đối chiếu:
        </label>
        <select
          id="scenario-select"
          className="select-control scenario-dropdown"
          value={selectedScenarioId}
          disabled={disabled}
          onChange={(e) => {
            const found = PRECONFIGURED_SCENARIOS.find((s) => s.id === e.target.value);
            if (found) onSelectScenario(found);
          }}
        >
          {PRECONFIGURED_SCENARIOS.map((scenario) => (
            <option key={scenario.id} value={scenario.id}>
              {scenario.name}
            </option>
          ))}
        </select>
      </div>

      <div className="scenario-info-card">
        <div className="scenario-desc">
          <strong>Mục tiêu kịch bản:</strong> {currentScenario.description}
        </div>
        {currentScenario.recommendedSources.length > 0 && (
          <div className="scenario-tags">
            <span className="tags-label">Nguồn đề xuất:</span>
            {currentScenario.recommendedSources.map((src, i) => (
              <span
                key={i}
                className={`tag-pill ${src.required ? "tag-required" : "tag-optional"}`}
                title={src.description}
              >
                {src.title} {src.required ? "(Bắt buộc)" : "(Tùy chọn)"}
              </span>
            ))}
          </div>
        )}
      </div>
    </section>
  );
};
