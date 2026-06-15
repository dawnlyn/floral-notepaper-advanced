import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { DiffEditor } from "./DiffEditor";
import { showToast } from "./Toast";
import {
  getSyncConflictDetail,
  listSyncConflicts,
  resolveAllSyncConflicts,
  resolveSyncConflict,
} from "../features/sync/api";
import type { PendingConflict, SyncConflictDetailDto } from "../features/sync/types";

interface ConflictResolutionModalProps {
  open: boolean;
  onClose: () => void;
  onResolved?: () => void;
}

interface MergeDraft {
  title: string;
  category: string;
  content: string;
}

function formatDateTime(value: string, t: TFunction): string {
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value || t("sync.conflict.empty");
  }
}

export function ConflictResolutionModal({
  open,
  onClose,
  onResolved,
}: ConflictResolutionModalProps) {
  const { t } = useTranslation();
  const [conflicts, setConflicts] = useState<PendingConflict[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<SyncConflictDetailDto | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [resolving, setResolving] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, MergeDraft>>({});
  const [bulkConfirm, setBulkConfirm] = useState<"local" | "remote" | null>(null);

  const loadConflicts = useCallback(async () => {
    try {
      const list = await listSyncConflicts();
      setConflicts(list.filter((c) => !c.resolved));
    } catch (error) {
      showToast(
        t("sync.conflict.resolveError", {
          message: error instanceof Error ? error.message : String(error),
          defaultValue: "Failed to load conflicts",
        }),
      );
    }
  }, [t]);

  useEffect(() => {
    if (!open) {
      setConflicts([]);
      setSelectedId(null);
      setDetail(null);
      setDrafts({});
      setBulkConfirm(null);
      return;
    }
    void loadConflicts();
  }, [open, loadConflicts]);

  useEffect(() => {
    if (!open || !selectedId) {
      setDetail(null);
      return;
    }
    setLoadingDetail(true);
    getSyncConflictDetail(selectedId)
      .then((d) => {
        setDetail(d);
        setDrafts((prev) => ({
          ...prev,
          [d.noteId]: prev[d.noteId] ?? {
            title: d.localTitle,
            category: d.localCategory,
            content: d.localContent,
          },
        }));
      })
      .catch((error) => {
        showToast(
          t("sync.conflict.resolveError", {
            message: error instanceof Error ? error.message : String(error),
            defaultValue: "Failed to load conflict detail",
          }),
        );
      })
      .finally(() => setLoadingDetail(false));
  }, [open, selectedId, t]);

  useEffect(() => {
    if (conflicts.length > 0 && !selectedId) {
      setSelectedId(conflicts[0].noteId);
    }
  }, [conflicts, selectedId]);

  const groupedConflicts = useMemo(() => {
    const map = new Map<string, PendingConflict[]>();
    for (const conflict of conflicts) {
      const key =
        conflict.localCategory || t("main.category.uncategorized", { defaultValue: "未分类" });
      const group = map.get(key) ?? [];
      group.push(conflict);
      map.set(key, group);
    }
    return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b));
  }, [conflicts, t]);

  const selectedDraft = selectedId ? drafts[selectedId] : null;

  const updateDraft = useCallback(
    (patch: Partial<MergeDraft>) => {
      if (!selectedId) return;
      setDrafts((prev) => ({
        ...prev,
        [selectedId]: {
          ...(prev[selectedId] ?? { title: "", category: "", content: "" }),
          ...patch,
        },
      }));
    },
    [selectedId],
  );

  const handleResolve = useCallback(
    async (choice: "local" | "remote" | "merge") => {
      if (!detail || !selectedDraft) return;
      setResolving(true);
      try {
        await resolveSyncConflict({
          noteId: detail.noteId,
          choice,
          mergedTitle: choice === "merge" ? selectedDraft.title : undefined,
          mergedCategory: choice === "merge" ? selectedDraft.category : undefined,
          mergedContent: choice === "merge" ? selectedDraft.content : undefined,
        });
        showToast(t("sync.conflict.resolveSuccess", { defaultValue: "Conflict resolved" }));
        await loadConflicts();
        onResolved?.();
        setSelectedId((current) => {
          const index = conflicts.findIndex((c) => c.noteId === current);
          const next = conflicts[index + 1] ?? conflicts[0];
          return next && next.noteId !== current ? next.noteId : null;
        });
      } catch (error) {
        showToast(
          t("sync.conflict.resolveError", {
            message: error instanceof Error ? error.message : String(error),
            defaultValue: "Failed to resolve conflict",
          }),
        );
      } finally {
        setResolving(false);
      }
    },
    [detail, selectedDraft, conflicts, loadConflicts, onResolved, t],
  );

  const handleResolveAll = useCallback(
    async (strategy: "local" | "remote") => {
      setResolving(true);
      try {
        await resolveAllSyncConflicts(strategy);
        showToast(t("sync.conflict.resolveSuccess", { defaultValue: "All conflicts resolved" }));
        await loadConflicts();
        onResolved?.();
      } catch (error) {
        showToast(
          t("sync.conflict.resolveError", {
            message: error instanceof Error ? error.message : String(error),
            defaultValue: "Failed to resolve conflicts",
          }),
        );
      } finally {
        setResolving(false);
        setBulkConfirm(null);
      }
    },
    [loadConflicts, onResolved, t],
  );

  const conflictTypeBadges = useCallback(
    (type: PendingConflict["conflictType"]) => {
      const entries: {
        key: keyof PendingConflict["conflictType"];
        label: string;
        color: string;
      }[] = [
        {
          key: "contentModified",
          label: t("sync.conflict.type.contentModified"),
          color: "bg-blue-500/10 text-blue-700",
        },
        {
          key: "titleChanged",
          label: t("sync.conflict.type.titleChanged"),
          color: "bg-amber-500/10 text-amber-700",
        },
        {
          key: "categoryMoved",
          label: t("sync.conflict.type.categoryMoved"),
          color: "bg-purple-500/10 text-purple-700",
        },
        {
          key: "deletedLocally",
          label: t("sync.conflict.type.deletedLocally"),
          color: "bg-red-500/10 text-red-700",
        },
        {
          key: "deletedRemotely",
          label: t("sync.conflict.type.deletedRemotely"),
          color: "bg-red-500/10 text-red-700",
        },
      ];
      return entries
        .filter((e) => type[e.key])
        .map((e) => (
          <span key={e.key} className={`px-1.5 py-0.5 rounded text-[10px] ${e.color}`}>
            {e.label}
          </span>
        ));
    },
    [t],
  );

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-ink/20 backdrop-blur-sm">
      <div className="w-full max-w-5xl h-[85vh] bg-cloud/92 backdrop-blur-sm border border-paper-deep/30 rounded-2xl shadow-2xl flex overflow-hidden">
        {/* Left sidebar: conflict list */}
        <div className="w-64 shrink-0 flex flex-col border-r border-paper-deep/30">
          <div className="flex items-center justify-between px-4 h-11 border-b border-paper-deep/25">
            <h2 className="text-[13px] font-display font-medium text-ink-soft">
              {t("sync.conflict.title")}
            </h2>
            <button
              type="button"
              onClick={onClose}
              className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
              title={t("sync.conflict.close")}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 12 12"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              >
                <path d="M2 2l8 8M10 2l-8 8" />
              </svg>
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-3">
            {conflicts.length === 0 ? (
              <div className="text-center text-[12px] text-ink-ghost py-8">
                {t("sync.conflict.empty")}
              </div>
            ) : (
              groupedConflicts.map(([category, items]) => (
                <div key={category}>
                  <div className="px-2 py-1 text-[10px] font-medium text-ink-ghost/80 truncate">
                    {category}
                  </div>
                  <div className="space-y-1">
                    {items.map((conflict) => {
                      const active = conflict.noteId === selectedId;
                      return (
                        <button
                          key={conflict.noteId}
                          type="button"
                          onClick={() => setSelectedId(conflict.noteId)}
                          className={`w-full text-left px-2.5 py-2 rounded-lg border transition-colors cursor-pointer ${
                            active
                              ? "bg-bamboo-mist/70 border-bamboo/30"
                              : "bg-transparent border-transparent hover:bg-paper-warm/60"
                          }`}
                        >
                          <div className="text-[12px] font-medium text-ink-soft truncate">
                            {conflict.title || t("common.untitledNote")}
                          </div>
                          <div className="flex flex-wrap gap-1 mt-1">
                            {conflictTypeBadges(conflict.conflictType)}
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))
            )}
          </div>

          {conflicts.length > 0 && (
            <div className="p-3 border-t border-paper-deep/25 space-y-2">
              <button
                type="button"
                disabled={resolving}
                onClick={() => setBulkConfirm("local")}
                className="w-full h-8 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 transition-colors cursor-pointer"
              >
                {t("sync.conflict.resolveAllLocal")}
              </button>
              <button
                type="button"
                disabled={resolving}
                onClick={() => setBulkConfirm("remote")}
                className="w-full h-8 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 transition-colors cursor-pointer"
              >
                {t("sync.conflict.resolveAllRemote")}
              </button>
            </div>
          )}
        </div>

        {/* Right detail panel */}
        <div className="flex-1 flex flex-col min-w-0">
          {conflicts.length === 0 ? (
            <div className="flex-1 flex items-center justify-center text-[13px] text-ink-ghost">
              {t("sync.conflict.empty")}
            </div>
          ) : loadingDetail || !detail ? (
            <div className="flex-1 flex items-center justify-center text-[13px] text-ink-ghost">
              {t("settings.sync.syncing", { defaultValue: "Loading..." })}
            </div>
          ) : (
            <>
              <div className="px-5 py-3 border-b border-paper-deep/25 space-y-3">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.field.title")} — {t("sync.conflict.localVersion")}
                    </label>
                    <div
                      className={`text-[12px] px-2.5 py-1.5 rounded-lg border ${
                        detail.localTitle !== detail.remoteTitle
                          ? "bg-red-500/10 border-red-400/30 text-red-700"
                          : "bg-paper-warm/50 border-paper-deep/30 text-ink-soft"
                      }`}
                    >
                      {detail.localTitle || t("common.untitledNote")}
                    </div>
                  </div>
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.field.title")} — {t("sync.conflict.remoteVersion")}
                    </label>
                    <div
                      className={`text-[12px] px-2.5 py-1.5 rounded-lg border ${
                        detail.localTitle !== detail.remoteTitle
                          ? "bg-green-500/10 border-green-400/30 text-green-700"
                          : "bg-paper-warm/50 border-paper-deep/30 text-ink-soft"
                      }`}
                    >
                      {detail.remoteTitle || t("common.untitledNote")}
                    </div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.field.category")} — {t("sync.conflict.localVersion")}
                    </label>
                    <div
                      className={`text-[12px] px-2.5 py-1.5 rounded-lg border ${
                        detail.localCategory !== detail.remoteCategory
                          ? "bg-red-500/10 border-red-400/30 text-red-700"
                          : "bg-paper-warm/50 border-paper-deep/30 text-ink-soft"
                      }`}
                    >
                      {detail.localCategory ||
                        t("main.category.uncategorized", { defaultValue: "未分类" })}
                    </div>
                  </div>
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.field.category")} — {t("sync.conflict.remoteVersion")}
                    </label>
                    <div
                      className={`text-[12px] px-2.5 py-1.5 rounded-lg border ${
                        detail.localCategory !== detail.remoteCategory
                          ? "bg-green-500/10 border-green-400/30 text-green-700"
                          : "bg-paper-warm/50 border-paper-deep/30 text-ink-soft"
                      }`}
                    >
                      {detail.remoteCategory ||
                        t("main.category.uncategorized", { defaultValue: "未分类" })}
                    </div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.field.updatedAt")} — {t("sync.conflict.localVersion")}
                    </label>
                    <div className="text-[12px] text-ink-soft px-2.5 py-1.5 rounded-lg bg-paper-warm/50 border border-paper-deep/30">
                      {formatDateTime(detail.localUpdatedAt, t)}
                    </div>
                  </div>
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.field.updatedAt")} — {t("sync.conflict.remoteVersion")}
                    </label>
                    <div className="text-[12px] text-ink-soft px-2.5 py-1.5 rounded-lg bg-paper-warm/50 border border-paper-deep/30">
                      {formatDateTime(detail.remoteUpdatedAt, t)}
                    </div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.mergedVersion")} — {t("sync.conflict.field.title")}
                    </label>
                    <input
                      type="text"
                      value={selectedDraft?.title ?? detail.localTitle}
                      onChange={(e) => updateDraft({ title: e.target.value })}
                      className="w-full h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft outline-none focus:border-bamboo/40"
                    />
                  </div>
                  <div>
                    <label className="block text-[10px] text-ink-ghost mb-1">
                      {t("sync.conflict.mergedVersion")} — {t("sync.conflict.field.category")}
                    </label>
                    <input
                      type="text"
                      value={selectedDraft?.category ?? detail.localCategory}
                      onChange={(e) => updateDraft({ category: e.target.value })}
                      className="w-full h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft outline-none focus:border-bamboo/40"
                    />
                  </div>
                </div>
              </div>

              <div className="flex-1 min-h-0 px-5 py-3">
                <DiffEditor
                  localContent={detail.localContent}
                  remoteContent={detail.remoteContent}
                  mergedContent={selectedDraft?.content ?? detail.localContent}
                  onChangeMerged={(content) => updateDraft({ content })}
                  copyLocalLabel={t("sync.conflict.chooseLocal")}
                  copyRemoteLabel={t("sync.conflict.chooseRemote")}
                  mergedLabel={t("sync.conflict.mergedVersion")}
                />
              </div>

              <div className="flex items-center justify-end gap-2 px-5 py-3 border-t border-paper-deep/25">
                <button
                  type="button"
                  disabled={resolving}
                  onClick={() => handleResolve("local")}
                  className="h-8 px-4 rounded-lg border border-paper-deep/45 text-[12px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 transition-colors cursor-pointer"
                >
                  {t("sync.conflict.chooseLocal")}
                </button>
                <button
                  type="button"
                  disabled={resolving}
                  onClick={() => handleResolve("remote")}
                  className="h-8 px-4 rounded-lg border border-paper-deep/45 text-[12px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 disabled:opacity-50 transition-colors cursor-pointer"
                >
                  {t("sync.conflict.chooseRemote")}
                </button>
                <button
                  type="button"
                  disabled={resolving}
                  onClick={() => handleResolve("merge")}
                  className="h-8 px-4 rounded-lg bg-bamboo text-white text-[12px] font-medium hover:bg-bamboo/90 disabled:opacity-50 transition-colors cursor-pointer"
                >
                  {t("sync.conflict.resolve")}
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      {bulkConfirm && (
        <div className="fixed inset-0 z-[110] flex items-center justify-center bg-ink/30 backdrop-blur-sm">
          <div className="w-80 bg-cloud/95 backdrop-blur-sm border border-paper-deep/40 rounded-xl p-4 shadow-xl">
            <p className="text-[13px] text-ink-soft mb-4">
              {bulkConfirm === "local"
                ? t("sync.conflict.resolveAllLocal")
                : t("sync.conflict.resolveAllRemote")}
              ？
            </p>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setBulkConfirm(null)}
                className="h-8 px-3 rounded-lg border border-paper-deep/45 text-[12px] text-ink-faint hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                disabled={resolving}
                onClick={() => void handleResolveAll(bulkConfirm)}
                className="h-8 px-3 rounded-lg bg-bamboo text-white text-[12px] font-medium hover:bg-bamboo/90 disabled:opacity-50 transition-colors cursor-pointer"
              >
                {t("sync.conflict.resolve")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
