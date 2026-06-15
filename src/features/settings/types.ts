export type ViewMode = "edit" | "split" | "preview";

export type ThemeOption = "light" | "dark" | "system";

export type TileColorMode = "system" | "custom";
export type BackgroundFit = "cover" | "contain" | "repeat";

export type OssProvider = "aliyun-oss" | "";

export type SyncInterval = "off" | "1min" | "3min" | "5min" | "10min" | "30min" | "1hour" | "daily";

export type SyncStrategy = "localWins" | "remoteWins" | "manual";

export interface AppConfig {
  locale: string;
  notesDir: string;
  globalShortcut: string;
  closeToTray: boolean;
  autostart: boolean;
  defaultViewMode: string;
  noteAutoSave: boolean;
  noteSurfaceAutoSave: boolean;
  tileColor: string;
  tileColorMode: TileColorMode;
  theme: ThemeOption;
  fontSize: number;
  surfaceFontSize: number;
  tabIndentSize: number;
  externalFileAutoSave: boolean;
  rememberSurfaceSize: boolean;
  tileCtrlClose: boolean;
  tileRenderMarkdown: boolean;
  renderHtmlMarkdown: boolean;
  surfaceWidth?: number;
  surfaceHeight?: number;
  toggleVisibilityShortcut: string;
  openAtCursor: boolean;
  backgroundImagePath?: string;
  backgroundFit?: BackgroundFit;
  backgroundDim?: number;
  backgroundBlur?: number;
  backgroundScale?: number;
  backgroundPositionX?: number;
  backgroundPositionY?: number;

  // OSS 云同步配置
  ossProvider: OssProvider;
  ossEndpoint: string;
  ossBucket: string;
  ossAccessKeyId: string;
  ossRemotePrefix: string;

  // 同步设置
  syncOnStartup: boolean;
  syncInterval: SyncInterval;
  syncStrategy: SyncStrategy;
  categoryOrder: string[];
}
