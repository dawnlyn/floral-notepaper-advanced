import { invoke } from "@tauri-apps/api/core";
import type {
  ConflictResolution,
  NoteCloudStatus,
  PendingConflict,
  SyncConflictDetailDto,
  SyncResultDto,
  SyncStatusDto,
} from "./types";

export function syncNow(): Promise<SyncResultDto> {
  return invoke("sync_now_command");
}

export function getSyncStatus(): Promise<SyncStatusDto> {
  return invoke("sync_status_command");
}

export function listSyncConflicts(): Promise<PendingConflict[]> {
  return invoke("sync_conflicts_list_command");
}

export function getSyncConflictDetail(noteId: string): Promise<SyncConflictDetailDto> {
  return invoke("sync_conflict_detail_command", { noteId });
}

export function resolveSyncConflict(resolution: ConflictResolution): Promise<void> {
  return invoke("sync_conflict_resolve_command", { resolution });
}

export function resolveAllSyncConflicts(strategy: "local" | "remote"): Promise<void> {
  return invoke("sync_conflict_resolve_all_command", { strategy });
}

export function testOssConnection(
  endpoint: string,
  bucket: string,
  accessKeyId: string,
  accessKeySecret: string,
): Promise<void> {
  return invoke("oss_test_connection_command", {
    endpoint,
    bucket,
    accessKeyId,
    accessKeySecret,
  });
}

export function saveOssCredential(accessKeyId: string, secret: string): Promise<void> {
  return invoke("oss_save_credential_command", { accessKeyId, secret });
}

export function getOssCredential(accessKeyId: string): Promise<string> {
  return invoke("oss_get_credential_command", { accessKeyId });
}

export function getNotesCloudStatus(): Promise<NoteCloudStatus[]> {
  return invoke("sync_get_cloud_status_command");
}
