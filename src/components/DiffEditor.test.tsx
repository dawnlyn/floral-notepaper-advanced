import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { DiffEditor, computeMergedContent, getInitialDecisions } from "./DiffEditor";
import type { DiffRow } from "./DiffEditor";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("DiffEditor", () => {
  test("renders local and remote content with selectable text", () => {
    const markup = renderToStaticMarkup(
      <DiffEditor
        localContent={"# Hello\n\nlocal line"}
        remoteContent={"# Hello\n\nremote line"}
        mergedContent={"# Hello\n\nlocal line"}
        onChangeMerged={vi.fn()}
      />,
    );

    expect(markup).toContain("local line");
    expect(markup).toContain("remote line");
    expect(markup).toContain("select-text");
  });

  test("renders merged title and category inputs above the textarea", () => {
    const markup = renderToStaticMarkup(
      <DiffEditor
        localContent={"# A"}
        remoteContent={"# B"}
        mergedContent={"# A"}
        onChangeMerged={vi.fn()}
        mergedTitle="Merged title"
        mergedCategory="Merged category"
        onChangeTitle={vi.fn()}
        onChangeCategory={vi.fn()}
      />,
    );

    expect(markup).toContain("Merged title");
    expect(markup).toContain("Merged category");
  });

  test("renders per-line action buttons for differing rows", () => {
    const markup = renderToStaticMarkup(
      <DiffEditor
        localContent={"local-only-line\nshared-line"}
        remoteContent={"shared-line"}
        mergedContent={"local-only-line\nshared-line"}
        onChangeMerged={vi.fn()}
      />,
    );

    expect(markup).toContain("选择");
    expect(markup).toContain("保留");
    expect(markup).toContain("<button");
  });
});

describe("computeMergedContent", () => {
  const rows: DiffRow[] = [
    { kind: "same", leftLine: 1, rightLine: 1, left: "shared", right: "shared" },
    { kind: "removed", leftLine: 2, left: "local-only", right: "" },
    { kind: "added", rightLine: 2, left: "", right: "remote-only" },
    { kind: "replace", leftLine: 3, rightLine: 3, left: "local", right: "remote" },
  ];

  test("keeps local lines by default for removed and replace rows", () => {
    const decisions = getInitialDecisions(rows);
    expect(computeMergedContent(rows, decisions)).toBe("shared\nlocal-only\nremote-only\nlocal");
  });

  test("skips removed line when decision is remote", () => {
    const decisions = { 1: "remote", 2: "remote", 3: "local" } as Record<
      number,
      "local" | "remote"
    >;
    expect(computeMergedContent(rows, decisions)).toBe("shared\nremote-only\nlocal");
  });

  test("uses remote content for replace row when decision is remote", () => {
    const decisions = { 1: "local", 2: "remote", 3: "remote" } as Record<
      number,
      "local" | "remote"
    >;
    expect(computeMergedContent(rows, decisions)).toBe("shared\nlocal-only\nremote-only\nremote");
  });
});
