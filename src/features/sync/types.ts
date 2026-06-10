export type SyncState = "idle" | "syncing" | "completed" | "error";

export interface SyncStatusDto {
  state: SyncState;
  lastSyncedAt: string | null;
  lastResult: SyncResultDto | null;
  nextSyncAt: string | null;
}

export interface SyncResultDto {
  uploaded: number;
  downloaded: number;
  conflicts: SyncConflictDto[];
  errors: string[];
  durationMs: number;
  completedAt: string;
}

export interface SyncConflictDto {
  noteId: string;
  noteTitle: string;
  localUpdatedAt: string;
  remoteUpdatedAt: string;
}

export interface NoteCloudStatus {
  noteId: string;
  synced: boolean;
  syncedAt: string | null;
  contentHash: string | null;
}
