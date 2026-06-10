import { invoke } from "@tauri-apps/api/core";
import type { NoteCloudStatus, SyncResultDto, SyncStatusDto } from "./types";

export function syncNow(): Promise<SyncResultDto> {
  return invoke("sync_now_command");
}

export function getSyncStatus(): Promise<SyncStatusDto> {
  return invoke("sync_status_command");
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
