import { invoke, isTauri } from "@tauri-apps/api/core";
export type Instance = {
  id: string;
  name: string;
  version: string;
  directory: string;
  min_ram: number;
  max_ram: number;
  width: number;
  height: number;
};
export type Settings = {
  game_directory: string;
  java_path: string;
  auto_java: boolean;
  min_ram: number;
  max_ram: number;
  width: number;
  height: number;
};
export type Saved = {
  schema: number;
  profile: { name: string; uuid: string; kind: string };
  settings: Settings;
  instances: Instance[];
  selected: string;
};
export type Version = { id: string; type: string; releaseTime: string };
export type Progress = {
  status: string;
  kind: string;
  stage: string;
  current: string;
  total_bytes: number;
  processed_bytes: number;
  downloaded_bytes: number;
  files_total: number;
  files_done: number;
  retries: number;
  speed: number;
  error: string | null;
};
export type Snapshot = {
  saved: Saved;
  progress: Progress;
  game: {
    running: boolean;
    pid: number | null;
    version: string;
    exit_code: number | null;
    error?: string | null;
  };
  logs: string[];
  installed: string[];
  root: string;
  runtime_os: string;
  runtime_arch: string;
  app_version: string;
};
export function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    return Promise.reject(
      new Error(
        "Abra o aplicativo desktop Cryon para acessar arquivos, versões e inicializar o Minecraft.",
      ),
    );
  return invoke<T>(name, args);
}
export const bytes = (n: number) =>
  n >= 1073741824
    ? (n / 1073741824).toFixed(2) + " GB"
    : n >= 1048576
      ? (n / 1048576).toFixed(1) + " MB"
      : n >= 1024
        ? (n / 1024).toFixed(1) + " KB"
        : n + " B";
