import type { ModuleId } from "@/stores/appStore";

export interface ModuleDef {
  readonly id: ModuleId;
  readonly label: string;
  readonly index: string;
  readonly shortcut: string;
  readonly tag: string;
}

export const MODULES: readonly ModuleDef[] = [
  { id: "website", label: "Website", index: "01", shortcut: "⌘1", tag: "MOD—01" },
  { id: "graphic2d", label: "Graphic 2D", index: "02", shortcut: "⌘2", tag: "MOD—02" },
  { id: "graphic3d", label: "Pseudo-3D", index: "03", shortcut: "⌘3", tag: "MOD—03" },
  { id: "video", label: "Video", index: "04", shortcut: "⌘4", tag: "MOD—04" },
  { id: "typography", label: "Type & Logo", index: "05", shortcut: "⌘5", tag: "MOD—05" },
] as const;

export function getModule(id: ModuleId): ModuleDef {
  const mod = MODULES.find((m) => m.id === id);
  if (!mod) throw new Error(`unknown module: ${id}`);
  return mod;
}
