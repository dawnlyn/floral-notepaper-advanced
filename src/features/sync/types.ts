export type SyncState = "idle" | "syncing" | "completed" | "error";

export interface ConflictType {
  contentModified: boolean;
  titleChanged: boolean;
  categoryMoved: boolean;
  deletedLocally: boolean;
  deletedRemotely: boolean;
}

export interface SyncConflictDto {
  noteId: string;
  noteTitle: string;
  localUpdatedAt: string;
  remoteUpdatedAt: string;
  conflictType: ConflictType;
}

export interface SyncResultDto {
  uploaded: number;
  downloaded: number;
  conflicts: SyncConflictDto[];
  errors: string[];
  durationMs: number;
  completedAt: string;
}

export interface SyncStatusDto {
  state: SyncState;
  lastSyncedAt: string | null;
  lastResult: SyncResultDto | null;
  nextSyncAt: string | null;
}

export interface PendingConflict {
  noteId: string;
  title: string;
  localTitle: string;
  remoteTitle: string;
  localCategory: string;
  remoteCategory: string;
  localUpdatedAt: string;
  remoteUpdatedAt: string;
  conflictType: ConflictType;
  resolved: boolean;
}

export interface SyncConflictDetailDto {
  noteId: string;
  localTitle: string;
  remoteTitle: string;
  localCategory: string;
  remoteCategory: string;
  localUpdatedAt: string;
  remoteUpdatedAt: string;
  localContent: string;
  remoteContent: string;
  conflictType: ConflictType;
}

export interface ConflictResolution {
  noteId: string;
  choice: "local" | "remote" | "merge";
  mergedTitle?: string;
  mergedCategory?: string;
  mergedContent?: string;
}

export interface NoteCloudStatus {
  noteId: string;
  synced: boolean;
  syncedAt: string | null;
  contentHash: string | null;
}
