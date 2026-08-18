import React from "react";

interface HeaderProps {
  onLoadDemoData: () => void;
  onResetSession: () => void;
  hasResults: boolean;
}

export const Header: React.FC<HeaderProps> = ({
  onLoadDemoData,
  onResetSession,
  hasResults,
}) => {
  return (
    <header className="app-header">
      <div className="header-brand">
        <div className="brand-logo">
          <svg
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <polyline points="14 2 14 8 20 8"></polyline>
            <line x1="16" y1="13" x2="8" y2="13"></line>
            <line x1="16" y1="17" x2="8" y2="17"></line>
            <polyline points="10 9 9 9 8 9"></polyline>
          </svg>
        </div>
        <div className="brand-text">
          <h1 className="brand-title">Đối chiếu số liệu kế toán</h1>
          <p className="brand-subtitle">
            Đối chiếu nhiều nguồn Excel hoàn toàn trên máy tính của bạn • 100% Offline & Bảo mật
          </p>
        </div>
      </div>

      <div className="header-actions">
        <span className="offline-badge" title="Tất cả dữ liệu xử lý an toàn tại máy cục bộ, không gửi ra Internet">
          <span className="dot-green"></span>
          Offline Bảo mật
        </span>

        <button
          type="button"
          className="btn btn-secondary"
          onClick={onLoadDemoData}
          title="Tải bộ dữ liệu mẫu để thử nghiệm ngay"
        >
          📁 Nạp dữ liệu mẫu
        </button>

        {hasResults && (
          <button
            type="button"
            className="btn btn-outline"
            onClick={onResetSession}
            title="Làm mới phiên đối chiếu"
          >
            🔄 Tạo phiên mới
          </button>
        )}
      </div>
    </header>
  );
};
