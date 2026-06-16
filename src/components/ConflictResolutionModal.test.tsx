import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { ConflictResolutionModal } from "./ConflictResolutionModal";
import { MOCK_CONFLICT_DATA } from "../features/sync/mockConflicts";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("ConflictResolutionModal", () => {
  test("renders the conflict resolution dialog when open", () => {
    const markup = renderToStaticMarkup(<ConflictResolutionModal open={true} onClose={vi.fn()} />);

    expect(markup).toContain("解决同步冲突");
    expect(markup).toContain("没有待处理的冲突");
  });

  test("renders mock conflicts when mockData is provided", () => {
    const markup = renderToStaticMarkup(
      <ConflictResolutionModal open={true} onClose={vi.fn()} mockData={MOCK_CONFLICT_DATA} />,
    );

    expect(markup).toContain("解决同步冲突");
    expect(markup).toContain("项目计划");
    expect(markup).toContain("购物清单");
    expect(markup).toContain("季度复盘");
    expect(markup).toContain("使用本地");
    expect(markup).toContain("使用云端");
  });
});
