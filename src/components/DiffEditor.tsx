import { useMemo } from "react";
import { diffLines } from "diff";

export interface DiffEditorProps {
  localContent: string;
  remoteContent: string;
  mergedContent: string;
  onChangeMerged: (value: string) => void;
  copyLocalLabel?: string;
  copyRemoteLabel?: string;
  mergedLabel?: string;
}

type DiffRow =
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

export function DiffEditor({
  localContent,
  remoteContent,
  mergedContent,
  onChangeMerged,
  copyLocalLabel = "Use local",
  copyRemoteLabel = "Use remote",
  mergedLabel = "Merged",
}: DiffEditorProps) {
  const rows = useMemo(() => buildRows(localContent, remoteContent), [localContent, remoteContent]);

  return (
    <div className="flex flex-col gap-3 h-full min-h-0">
      <div className="flex-1 min-h-0 border border-paper-deep/30 rounded-xl overflow-hidden flex bg-paper-warm/30">
        <div className="flex-1 flex flex-col min-w-0 border-r border-paper-deep/30">
          <div className="px-3 py-1.5 text-[11px] font-medium text-ink-faint bg-paper-warm/60 border-b border-paper-deep/30">
            {copyLocalLabel}
          </div>
          <div className="flex-1 overflow-auto p-2 space-y-[1px]">
            {rows.map((row, index) => (
              <div
                key={`l-${index}`}
                className={`flex text-[12px] leading-5 font-mono ${
                  row.kind === "removed" || row.kind === "replace"
                    ? "bg-red-500/10 text-red-700"
                    : "text-ink-soft"
                }`}
              >
                <span className="w-8 shrink-0 text-right pr-2 text-ink-ghost/50 select-none">
                  {row.leftLine ?? ""}
                </span>
                <span className="whitespace-pre-wrap break-words flex-1">{row.left}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="flex-1 flex flex-col min-w-0">
          <div className="px-3 py-1.5 text-[11px] font-medium text-ink-faint bg-paper-warm/60 border-b border-paper-deep/30">
            {copyRemoteLabel}
          </div>
          <div className="flex-1 overflow-auto p-2 space-y-[1px]">
            {rows.map((row, index) => (
              <div
                key={`r-${index}`}
                className={`flex text-[12px] leading-5 font-mono ${
                  row.kind === "added" || row.kind === "replace"
                    ? "bg-green-500/10 text-green-700"
                    : "text-ink-soft"
                }`}
              >
                <span className="w-8 shrink-0 text-right pr-2 text-ink-ghost/50 select-none">
                  {row.rightLine ?? ""}
                </span>
                <span className="whitespace-pre-wrap break-words flex-1">{row.right}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="shrink-0 flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-medium text-ink-faint">{mergedLabel}</span>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => onChangeMerged(localContent)}
              className="h-7 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              {copyLocalLabel}
            </button>
            <button
              type="button"
              onClick={() => onChangeMerged(remoteContent)}
              className="h-7 px-2.5 rounded-lg border border-paper-deep/45 text-[11px] text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-colors cursor-pointer"
            >
              {copyRemoteLabel}
            </button>
          </div>
        </div>
        <textarea
          value={mergedContent}
          onChange={(e) => onChangeMerged(e.target.value)}
          spellCheck={false}
          className="w-full h-32 resize-none rounded-xl bg-paper-warm/50 border border-paper-deep/40 p-3 text-[12px] leading-5 font-mono text-ink-soft outline-none focus:border-bamboo/40 placeholder:text-ink-ghost/60"
        />
      </div>
    </div>
  );
}
