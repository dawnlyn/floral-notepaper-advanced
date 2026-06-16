import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { diffLines } from "diff";

export interface DiffEditorProps {
  localContent: string;
  remoteContent: string;
  mergedContent: string;
  onChangeMerged: (value: string) => void;
  copyLocalLabel?: string;
  copyRemoteLabel?: string;
  mergedLabel?: string;
  mergedTitle?: string;
  mergedCategory?: string;
  onChangeTitle?: (value: string) => void;
  onChangeCategory?: (value: string) => void;
  titleLabel?: string;
  categoryLabel?: string;
}

export type DiffRow =
  | {
      kind: "same";
      leftLine: number;
      rightLine: number;
      left: string;
      right: string;
    }
  | {
      kind: "removed";
      leftLine: number;
      rightLine?: undefined;
      left: string;
      right: "";
    }
  | {
      kind: "added";
      leftLine?: undefined;
      rightLine: number;
      left: "";
      right: string;
    }
  | {
      kind: "replace";
      leftLine: number;
      rightLine: number;
      left: string;
      right: string;
    };

function splitChangeValue(value: string): string[] {
  if (value === "") return [""];
  const lines = value.split("\n");
  if (value.endsWith("\n")) {
    lines.pop();
  }
  return lines;
}

function buildRows(local: string, remote: string): DiffRow[] {
  const changes = diffLines(local, remote);
  const rows: DiffRow[] = [];
  let leftLine = 1;
  let rightLine = 1;

  for (let i = 0; i < changes.length; i++) {
    const change = changes[i];
    const next = changes[i + 1];
    const lines = splitChangeValue(change.value);

    if (change.removed && next?.added) {
      const addedLines = splitChangeValue(next.value);
      const max = Math.max(lines.length, addedLines.length);
      for (let k = 0; k < max; k++) {
        rows.push({
          kind: "replace",
          leftLine: leftLine++,
          rightLine: rightLine++,
          left: lines[k] ?? "",
          right: addedLines[k] ?? "",
        });
      }
      i++;
      continue;
    }

    if (change.removed) {
      for (const line of lines) {
        rows.push({
          kind: "removed",
          leftLine: leftLine++,
          left: line,
          right: "",
        });
      }
      continue;
    }

    if (change.added) {
      for (const line of lines) {
        rows.push({
          kind: "added",
          rightLine: rightLine++,
          left: "",
          right: line,
        });
      }
      continue;
    }

    for (const line of lines) {
      rows.push({
        kind: "same",
        leftLine: leftLine++,
        rightLine: rightLine++,
        left: line,
        right: line,
      });
    }
  }

  return rows;
}

type LineDecision = "local" | "remote";

export function getInitialDecisions(rows: DiffRow[]): Record<number, LineDecision> {
  const decisions: Record<number, LineDecision> = {};
  rows.forEach((row, index) => {
    if (row.kind === "removed") {
      decisions[index] = "local";
    } else if (row.kind === "added") {
      decisions[index] = "remote";
    } else if (row.kind === "replace") {
      decisions[index] = "local";
    }
  });
  return decisions;
}

export function computeMergedContent(
  rows: DiffRow[],
  decisions: Record<number, LineDecision>,
): string {
  const lines: string[] = [];
  for (let i = 0; i < rows.length; i++) {
    const row = rows[i];
    const decision = decisions[i];

    if (row.kind === "same") {
      lines.push(row.left);
    } else if (row.kind === "removed") {
      if (decision === "local") {
        lines.push(row.left);
      }
    } else if (row.kind === "added") {
      if (decision === "remote") {
        lines.push(row.right);
      }
    } else if (row.kind === "replace") {
      if (decision === "local") {
        lines.push(row.left);
      } else {
        lines.push(row.right);
      }
    }
  }
  return lines.join("\n");
}

export function DiffEditor({
  localContent,
  remoteContent,
  mergedContent,
  onChangeMerged,
  copyLocalLabel,
  copyRemoteLabel,
  mergedLabel,
  mergedTitle = "",
  mergedCategory = "",
  onChangeTitle,
  onChangeCategory,
  titleLabel,
  categoryLabel,
}: DiffEditorProps) {
  const { t } = useTranslation();
  const rows = useMemo(() => buildRows(localContent, remoteContent), [localContent, remoteContent]);
  const [decisions, setDecisions] = useState<Record<number, LineDecision>>(() =>
    getInitialDecisions(rows),
  );
  const [isCustomContent, setIsCustomContent] = useState(false);

  const computedContent = useMemo(() => computeMergedContent(rows, decisions), [rows, decisions]);

  const onChangeMergedRef = useRef(onChangeMerged);
  onChangeMergedRef.current = onChangeMerged;

  useEffect(() => {
    setDecisions(getInitialDecisions(rows));
    setIsCustomContent(false);
  }, [localContent, remoteContent, rows]);

  useEffect(() => {
    if (!isCustomContent && computedContent !== mergedContent) {
      onChangeMergedRef.current(computedContent);
    }
  }, [computedContent, isCustomContent, mergedContent]);

  const setDecision = (index: number, decision: LineDecision) => {
    setDecisions((prev) => ({ ...prev, [index]: decision }));
    setIsCustomContent(false);
  };

  const applyAll = (decision: LineDecision) => {
    const next: Record<number, LineDecision> = {};
    rows.forEach((row, index) => {
      if (row.kind === "removed" || row.kind === "added" || row.kind === "replace") {
        next[index] = decision;
      }
    });
    setDecisions(next);
    setIsCustomContent(false);
  };

  const handleMergedChange = (value: string) => {
    setIsCustomContent(true);
    onChangeMergedRef.current(value);
  };

  const localLabel = copyLocalLabel ?? t("sync.conflict.chooseLocal", { defaultValue: "使用本地" });
  const remoteLabel =
    copyRemoteLabel ?? t("sync.conflict.chooseRemote", { defaultValue: "使用云端" });
  const mergedTitleLabel =
    mergedLabel ?? t("sync.conflict.mergedVersion", { defaultValue: "合并版本" });
  const fieldTitleLabel = titleLabel ?? t("sync.conflict.field.title", { defaultValue: "标题" });
  const fieldCategoryLabel =
    categoryLabel ?? t("sync.conflict.field.category", { defaultValue: "分类" });
  const keepLabel = t("sync.conflict.keepLine", { defaultValue: "保留" });
  const skipLabel = t("sync.conflict.skipLine", { defaultValue: "跳过" });

  const cellClass =
    "flex items-center gap-1 px-2 py-[1px] select-text whitespace-pre-wrap break-words flex-1";

  return (
    <div className="flex flex-col gap-3 h-full min-h-0">
      <div className="flex-1 min-h-0 border border-paper-deep/30 rounded-xl overflow-hidden flex flex-col bg-paper-warm/30">
        <div className="grid grid-cols-[1fr_72px_1fr] border-b border-paper-deep/30 bg-paper-warm/60">
          <div className="px-3 py-1.5 text-[11px] font-medium text-ink-faint truncate">
            {localLabel}
          </div>
          <div className="px-1 py-1.5 text-center text-[10px] font-medium text-ink-ghost/70 border-x border-paper-deep/20">
            {t("sync.conflict.lineAction", { defaultValue: "选择" })}
          </div>
          <div className="px-3 py-1.5 text-[11px] font-medium text-ink-faint truncate">
            {remoteLabel}
          </div>
        </div>

        <div className="flex-1 overflow-auto">
          {rows.map((row, index) => {
            const decision = decisions[index];
            const isLocalChosen = decision === "local";
            const isRemoteChosen = decision === "remote";

            return (
              <div
                key={index}
                className={`grid grid-cols-[1fr_72px_1fr] text-[12px] leading-5 font-mono border-b border-paper-deep/10 last:border-b-0 ${
                  row.kind === "removed" || row.kind === "replace"
                    ? "bg-red-500/[0.04]"
                    : row.kind === "added"
                      ? "bg-green-500/[0.04]"
                      : ""
                }`}
              >
                <div
                  className={`flex min-w-0 border-r border-paper-deep/10 ${
                    row.kind === "removed" || row.kind === "replace"
                      ? "bg-red-500/10 text-red-700"
                      : "text-ink-soft"
                  }`}
                >
                  <span className="w-8 shrink-0 text-right pr-2 pl-1 py-[1px] text-ink-ghost/50 select-none">
                    {row.leftLine ?? ""}
                  </span>
                  <span className={cellClass}>{row.left}</span>
                </div>

                <div className="flex items-center justify-center gap-0.5 border-x border-paper-deep/10 bg-paper-warm/40">
                  {row.kind === "same" && <span className="text-[10px] text-ink-ghost/50">—</span>}
                  {row.kind === "removed" && (
                    <button
                      type="button"
                      onClick={() => setDecision(index, isLocalChosen ? "remote" : "local")}
                      className={`px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors cursor-pointer ${
                        isLocalChosen
                          ? "bg-red-500/15 text-red-700 hover:bg-red-500/25"
                          : "text-ink-ghost/60 hover:bg-paper-deep/10 hover:text-ink-faint"
                      }`}
                      title={isLocalChosen ? keepLabel : skipLabel}
                    >
                      {isLocalChosen ? keepLabel : skipLabel}
                    </button>
                  )}
                  {row.kind === "added" && (
                    <button
                      type="button"
                      onClick={() => setDecision(index, isRemoteChosen ? "local" : "remote")}
                      className={`px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors cursor-pointer ${
                        isRemoteChosen
                          ? "bg-green-500/15 text-green-700 hover:bg-green-500/25"
                          : "text-ink-ghost/60 hover:bg-paper-deep/10 hover:text-ink-faint"
                      }`}
                      title={isRemoteChosen ? keepLabel : skipLabel}
                    >
                      {isRemoteChosen ? keepLabel : skipLabel}
                    </button>
                  )}
                  {row.kind === "replace" && (
                    <>
                      <button
                        type="button"
                        onClick={() => setDecision(index, "local")}
                        className={`px-1 py-0.5 rounded text-[10px] font-medium transition-colors cursor-pointer ${
                          isLocalChosen
                            ? "bg-red-500/15 text-red-700"
                            : "text-ink-ghost/60 hover:bg-paper-deep/10 hover:text-ink-faint"
                        }`}
                        title={localLabel}
                      >
                        {t("sync.conflict.localShort", { defaultValue: "本地" })}
                      </button>
                      <button
                        type="button"
                        onClick={() => setDecision(index, "remote")}
                        className={`px-1 py-0.5 rounded text-[10px] font-medium transition-colors cursor-pointer ${
                          isRemoteChosen
                            ? "bg-green-500/15 text-green-700"
                            : "text-ink-ghost/60 hover:bg-paper-deep/10 hover:text-ink-faint"
                        }`}
                        title={remoteLabel}
                      >
                        {t("sync.conflict.remoteShort", { defaultValue: "云端" })}
                      </button>
                    </>
                  )}
                </div>

                <div
                  className={`flex min-w-0 ${
                    row.kind === "added" || row.kind === "replace"
                      ? "bg-green-500/10 text-green-700"
                      : "text-ink-soft"
                  }`}
                >
                  <span className="w-8 shrink-0 text-right pr-2 pl-1 py-[1px] text-ink-ghost/50 select-none">
                    {row.rightLine ?? ""}
                  </span>
                  <span className={cellClass}>{row.right}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="shrink-0 flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-medium text-ink-faint">{mergedTitleLabel}</span>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => applyAll("local")}
              className="h-7 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              {localLabel}
            </button>
            <button
              type="button"
              onClick={() => applyAll("remote")}
              className="h-7 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              {remoteLabel}
            </button>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-[10px] text-ink-ghost mb-1">{fieldTitleLabel}</label>
            <input
              type="text"
              value={mergedTitle}
              onChange={(e) => onChangeTitle?.(e.target.value)}
              className="w-full h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft outline-none focus:border-bamboo/40"
            />
          </div>
          <div>
            <label className="block text-[10px] text-ink-ghost mb-1">{fieldCategoryLabel}</label>
            <input
              type="text"
              value={mergedCategory}
              onChange={(e) => onChangeCategory?.(e.target.value)}
              className="w-full h-8 px-2.5 rounded-lg bg-paper-warm/70 border border-paper-deep/40 text-[12px] text-ink-soft outline-none focus:border-bamboo/40"
            />
          </div>
        </div>

        <textarea
          value={mergedContent}
          onChange={(e) => handleMergedChange(e.target.value)}
          spellCheck={false}
          className="w-full h-32 resize-none rounded-xl bg-paper-warm/50 border border-paper-deep/40 p-3 text-[12px] leading-5 font-mono text-ink-soft outline-none focus:border-bamboo/40 placeholder:text-ink-ghost/60"
        />
      </div>
    </div>
  );
}
