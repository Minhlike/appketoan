/// <reference types="vite/client" />

interface Window {
  __APPKETOAN_BOOT__?: {
    mounted: boolean;
    reportBootFailure: (reason: unknown) => void;
  };
}
