import { t, type TFunction } from "i18next";
import type { Note, NoteMetadata } from "./types";

export function getDisplayTitle(
  note: Pick<NoteMetadata, "title" | "preview">,
  translate: TFunction = t,
): string {
  const title = note.title.trim();
  if (title) return title;

  const preview = note.preview.trim();
  if (preview) return preview.slice(0, 20);

  return translate("common.untitledNote", { defaultValue: "无标题笔记" });
}

export function buildPreview(content: string): string {
  return content.split(/\s+/).filter(Boolean).join(" ").slice(0, 80);
}

export function countNoteChars(content: string): number {
  let count = 0;
  for (const ch of content) {
    if (!/\s/.test(ch)) count++;
  }
  return count;
}

export function metadataFromNote(note: Note): NoteMetadata {
  return {
    id: note.id,
    title: note.title,
    fileName: note.fileName,
    category: note.category,
    createdAt: note.createdAt,
    updatedAt: note.updatedAt,
    wordCount: note.wordCount,
    preview: buildPreview(note.content),
  };
}

export interface CategoryGroup {
  category: string;
  notes: NoteMetadata[];
  latestUpdatedAt: string;
}

export interface CategoryTreeNode {
  category: string;
  name: string;
  notes: NoteMetadata[];
  latestUpdatedAt: string;
  children: CategoryTreeNode[];
}

function categoryOrderRank(category: string, order: string[] | undefined): number {
  if (!order) return Number.POSITIVE_INFINITY;
  const index = order.indexOf(category);
  return index === -1 ? Number.POSITIVE_INFINITY : index;
}

export function compareCategoryOrder(a: string, b: string, order: string[] | undefined): number {
  const rankA = categoryOrderRank(a, order);
  const rankB = categoryOrderRank(b, order);
  if (rankA !== rankB) {
    if (rankA === Number.POSITIVE_INFINITY) return 1;
    if (rankB === Number.POSITIVE_INFINITY) return -1;
    return rankA - rankB;
  }
  if (!a) return 1;
  if (!b) return -1;
  return a.localeCompare(b);
}

export function parentCategoryPath(category: string): string {
  const lastSlash = category.lastIndexOf("/");
  return lastSlash === -1 ? "" : category.slice(0, lastSlash);
}

export function buildCategoryTree(groups: CategoryGroup[]): CategoryTreeNode[] {
  const roots: CategoryTreeNode[] = [];

  for (const group of groups) {
    if (!group.category) continue;
    insertCategoryNode(roots, group);
  }

  return roots;
}

function insertCategoryNode(nodes: CategoryTreeNode[], group: CategoryGroup): void {
  const segments = group.category.split("/");
  let current = nodes;
  let path = "";

  for (let i = 0; i < segments.length; i++) {
    const name = segments[i];
    path = path ? `${path}/${name}` : name;
    let node = current.find((n) => n.name === name);
    if (!node) {
      node = {
        category: path,
        name,
        notes: [],
        latestUpdatedAt: "",
        children: [],
      };
      current.push(node);
    }
    if (i === segments.length - 1) {
      node.notes = group.notes;
      node.latestUpdatedAt = group.latestUpdatedAt;
    }
    current = node.children;
  }
}

export function groupNotesByCategory(
  notes: NoteMetadata[],
  allCategories: string[] = [],
  categoryOrder: string[] = [],
): CategoryGroup[] {
  const groups = new Map<string, NoteMetadata[]>();

  for (const cat of allCategories) {
    groups.set(cat, []);
  }

  for (const note of notes) {
    const key = note.category || "";
    const list = groups.get(key);
    if (list) {
      list.push(note);
    } else {
      groups.set(key, [note]);
    }
  }

  const result: CategoryGroup[] = [];
  for (const [category, categoryNotes] of groups) {
    categoryNotes.sort((a, b) => {
      const orderA = a.order ?? 0;
      const orderB = b.order ?? 0;
      if (orderA !== orderB) return orderA - orderB;
      return b.updatedAt.localeCompare(a.updatedAt);
    });
    result.push({
      category,
      notes: categoryNotes,
      latestUpdatedAt: categoryNotes[0]?.updatedAt ?? "",
    });
  }

  result.sort((a, b) => compareCategoryOrder(a.category, b.category, categoryOrder));
  return result;
}

export function filterNotes(notes: NoteMetadata[], query: string): NoteMetadata[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return notes;

  return notes.filter((note) => {
    const haystack = [note.title, note.preview, note.fileName, getDisplayTitle(note)]
      .join(" ")
      .toLowerCase();
    return haystack.includes(normalized);
  });
}

export function formatShortDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "--";
  return `${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function countCategoryNotes(node: CategoryTreeNode): number {
  let count = node.notes.length;
  for (const child of node.children) {
    count += countCategoryNotes(child);
  }
  return count;
}

export function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "--:--";
  return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}
