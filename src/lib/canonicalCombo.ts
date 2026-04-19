/**
 * Canonicalize a KeyboardEvent into a stable string.
 * `Mod+` collapses macOS Meta + Linux/Windows Ctrl into one platform-agnostic
 * prefix so shortcut definitions don't have to branch.
 *
 * Letters are uppercased; Enter / Escape / numeric / punctuation pass through.
 */
export function eventToCombo(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.metaKey || e.ctrlKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  let key = e.key;
  if (key.length === 1 && key >= "a" && key <= "z") key = key.toUpperCase();
  parts.push(key);
  return parts.join("+");
}
