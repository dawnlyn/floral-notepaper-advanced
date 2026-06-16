import type { PendingConflict, SyncConflictDetailDto } from "./types";

const now = new Date();
const oneHourAgo = new Date(now.getTime() - 60 * 60 * 1000).toISOString();
const twoHoursAgo = new Date(now.getTime() - 2 * 60 * 60 * 1000).toISOString();
const oneDayAgo = new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString();

export const MOCK_CONFLICTS: PendingConflict[] = [
  {
    noteId: "mock-note-project-plan",
    title: "项目计划",
    localTitle: "项目计划",
    remoteTitle: "项目计划（云端）",
    localCategory: "工作/2024",
    remoteCategory: "工作/2025",
    localUpdatedAt: twoHoursAgo,
    remoteUpdatedAt: oneHourAgo,
    conflictType: {
      contentModified: true,
      titleChanged: true,
      categoryMoved: true,
      deletedLocally: false,
      deletedRemotely: false,
    },
    resolved: false,
  },
  {
    noteId: "mock-note-shopping-list",
    title: "购物清单",
    localTitle: "购物清单",
    remoteTitle: "购物清单",
    localCategory: "生活",
    remoteCategory: "生活",
    localUpdatedAt: oneDayAgo,
    remoteUpdatedAt: oneHourAgo,
    conflictType: {
      contentModified: true,
      titleChanged: false,
      categoryMoved: false,
      deletedLocally: false,
      deletedRemotely: false,
    },
    resolved: false,
  },
  {
    noteId: "mock-note-quarterly-review",
    title: "季度复盘",
    localTitle: "季度复盘",
    remoteTitle: "季度复盘",
    localCategory: "工作/2024/Q1",
    remoteCategory: "归档/2024/Q1",
    localUpdatedAt: twoHoursAgo,
    remoteUpdatedAt: oneHourAgo,
    conflictType: {
      contentModified: true,
      titleChanged: false,
      categoryMoved: true,
      deletedLocally: false,
      deletedRemotely: false,
    },
    resolved: false,
  },
];

export const MOCK_CONFLICT_DETAILS: Record<string, SyncConflictDetailDto> = {
  "mock-note-project-plan": {
    noteId: "mock-note-project-plan",
    localTitle: "项目计划",
    remoteTitle: "项目计划（云端）",
    localCategory: "工作/2024",
    remoteCategory: "工作/2025",
    localUpdatedAt: twoHoursAgo,
    remoteUpdatedAt: oneHourAgo,
    localContent: [
      "# 项目计划",
      "",
      "- [x] 完成需求分析",
      "- [x] 设计原型",
      "- [ ] 编写代码",
      "- [ ] 测试验收",
    ].join("\n"),
    remoteContent: [
      "# 项目计划（云端）",
      "",
      "- [x] 完成需求分析",
      "- [x] 设计原型",
      "- [x] 编写代码",
      "- [ ] 测试验收",
      "- [ ] 部署上线",
    ].join("\n"),
    conflictType: {
      contentModified: true,
      titleChanged: true,
      categoryMoved: true,
      deletedLocally: false,
      deletedRemotely: false,
    },
  },
  "mock-note-shopping-list": {
    noteId: "mock-note-shopping-list",
    localTitle: "购物清单",
    remoteTitle: "购物清单",
    localCategory: "生活",
    remoteCategory: "生活",
    localUpdatedAt: oneDayAgo,
    remoteUpdatedAt: oneHourAgo,
    localContent: ["# 购物清单", "", "- 牛奶", "- 鸡蛋", "- 面包"].join("\n"),
    remoteContent: ["# 购物清单", "", "- 牛奶", "- 鸡蛋", "- 面包", "- 黄油", "- 咖啡"].join("\n"),
    conflictType: {
      contentModified: true,
      titleChanged: false,
      categoryMoved: false,
      deletedLocally: false,
      deletedRemotely: false,
    },
  },
  "mock-note-quarterly-review": {
    noteId: "mock-note-quarterly-review",
    localTitle: "季度复盘",
    remoteTitle: "季度复盘",
    localCategory: "工作/2024/Q1",
    remoteCategory: "归档/2024/Q1",
    localUpdatedAt: twoHoursAgo,
    remoteUpdatedAt: oneHourAgo,
    localContent: ["# 季度复盘", "", "- 完成 3 个关键目标", "- 待改进：沟通效率"].join("\n"),
    remoteContent: [
      "# 季度复盘",
      "",
      "- 完成 3 个关键目标",
      "- 待改进：文档沉淀",
      "- 下季度重点：自动化",
    ].join("\n"),
    conflictType: {
      contentModified: true,
      titleChanged: false,
      categoryMoved: true,
      deletedLocally: false,
      deletedRemotely: false,
    },
  },
};

export const MOCK_CONFLICT_DATA = {
  conflicts: MOCK_CONFLICTS,
  details: MOCK_CONFLICT_DETAILS,
};
