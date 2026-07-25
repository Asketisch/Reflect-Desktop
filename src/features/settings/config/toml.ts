/**
 * Pure TOML helpers for complex sections, feature flags, and unknown
 * section detection. These functions operate purely on TOML strings
 * so they can be exercised from focused unit tests without React.
 *
 * Semantics (preserved from ConfigForm.tsx):
 *  - unknown top-level sections are surfaced via `findUnknownSections`
 *    so the UI can hint users to use the Advanced TOML editor
 *  - subsection edits keep unknown keys inside other sections intact
 *  - feature flags are stored as a `flags = { ... }` inline table
 */

export interface SubSectionEntry {
  name: string;
  body: string;
}

export interface FlagEntry {
  key: string;
  value: boolean;
}

/** Extract all subsection entries under a given prefix (e.g. `mcp_servers`). */
export function extractSubSections(toml: string, prefix: string): SubSectionEntry[] {
  const lines = toml.split('\n');
  const out: SubSectionEntry[] = [];
  let current: { name: string; lines: string[] } | null = null;
  for (const raw of lines) {
    const line = raw.trim();
    const sec = /^\[([^\]]+)\]/.exec(line);
    if (sec) {
      if (current) out.push({ name: current.name, body: current.lines.join('\n').trim() });
      const full = sec[1].trim();
      if (full === prefix) current = { name: '(default)', lines: [] };
      else if (full.startsWith(`${prefix}.`)) current = { name: full.slice(prefix.length + 1), lines: [] };
      else current = null;
    } else if (current) {
      current.lines.push(raw);
    }
  }
  if (current) out.push({ name: current.name, body: current.lines.join('\n').trim() });
  return out.filter((e) => e.body.length > 0);
}

/** Replace the body of a subsection (or the bare `[prefix]` section when name is `(default)`). */
export function updateSubSection(toml: string, prefix: string, name: string, body: string): string {
  if (name === '(default)') return setSectionBody(toml, `[${prefix}]`, body);
  return setSectionBody(toml, `[${prefix}.${name}]`, body);
}

/** Remove a subsection by name. */
export function removeSubSection(toml: string, prefix: string, name: string): string {
  const target = name === '(default)' ? `[${prefix}]` : `[${prefix}.${name}]`;
  return removeSection(toml, target);
}

/** Append a fresh subsection if it does not already exist. */
export function addSubSection(toml: string, prefix: string, name: string): string {
  const header = name.includes('.') ? `[${prefix}.${name}]` : `[${prefix}]`;
  if (toml.includes(header)) return toml;
  return `${toml.replace(/\s*$/, '')}\n\n${header}\n# TODO: edit me\n`;
}

/**
 * Replace the body lines under a header. If the header is not present
 * it is appended. Unknown lines (e.g. other sections) are preserved.
 */
export function setSectionBody(toml: string, header: string, body: string): string {
  const lines = toml.split('\n');
  const out: string[] = [];
  let replaced = false;
  let inTarget = false;
  let blockLines: string[] = [];
  for (const line of lines) {
    if (line.trim() === header) {
      if (inTarget) {
        out.push(...blockLines);
      }
      out.push(header);
      out.push(body.trimEnd());
      out.push('');
      inTarget = true;
      blockLines = [];
      replaced = true;
      continue;
    }
    if (inTarget) {
      if (/^\[/.test(line.trim())) {
        out.push(...blockLines);
        inTarget = false;
        blockLines = [];
      } else {
        blockLines.push(line);
        continue;
      }
    }
    out.push(line);
  }
  if (inTarget) out.push(...blockLines);
  if (!replaced) {
    out.push('');
    out.push(header);
    out.push(body.trimEnd());
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n');
}

/** Strip a single header and its body from the TOML. */
export function removeSection(toml: string, header: string): string {
  const lines = toml.split('\n');
  const out: string[] = [];
  let inTarget = false;
  for (const line of lines) {
    if (line.trim() === header) {
      inTarget = true;
      continue;
    }
    if (inTarget) {
      if (/^\[/.test(line.trim())) inTarget = false;
      else continue;
    }
    out.push(line);
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n');
}

/** Read all boolean flags declared inside `[feature_flags]`. */
export function extractFlags(toml: string): FlagEntry[] {
  const lines = toml.split('\n');
  let inSection = false;
  const out: FlagEntry[] = [];
  for (const raw of lines) {
    const line = raw.trim();
    const sec = /^\[([^\]]+)\]/.exec(line);
    if (sec) {
      inSection = sec[1].trim() === 'feature_flags';
      continue;
    }
    if (!inSection) continue;
    const flagsMatch = /^flags\s*=\s*\{(.+)\}$/.exec(line);
    if (flagsMatch) {
      const pairs = flagsMatch[1].split(',');
      for (const pair of pairs) {
        const kv = /^"?([\w.-]+)"?\s*=\s*(true|false)/.exec(pair.trim());
        if (kv) out.push({ key: kv[1], value: kv[2] === 'true' });
      }
    } else {
      const inline = /^"?([\w.-]+)"?\s*=\s*(true|false)/.exec(line);
      if (inline) out.push({ key: inline[1], value: inline[2] === 'true' });
    }
  }
  return out;
}

/** Set a single feature flag to the given boolean value, creating the section if needed. */
export function setFlag(toml: string, key: string, value: boolean): string {
  const lines = toml.split('\n');
  let start = -1;
  let end = lines.length;
  for (let i = 0; i < lines.length; i++) {
    const sec = /^\[([^\]]+)\]/.exec(lines[i].trim());
    if (sec) {
      if (start !== -1 && end === lines.length) end = i;
      if (sec[1].trim() === 'feature_flags' && start === -1) start = i;
    }
  }
  if (start === -1) {
    return `${toml.replace(/\s*$/, '')}\n\n[feature_flags]\nflags = { "${key}" = ${value} }\n`;
  }
  const body = lines.slice(start + 1, end);
  const flagsLine = body.find((l) => /^flags\s*=/.test(l.trim()));
  if (!flagsLine) {
    body.push(`flags = { "${key}" = ${value} }`);
    lines.splice(end, 0, ...body);
    return lines.join('\n');
  }
  const idx = body.indexOf(flagsLine);
  const inline = /\{(.+)\}/.exec(flagsLine);
  if (!inline) return toml;
  const entries = inline[1]
    .split(',')
    .map((p) => p.trim())
    .filter(Boolean);
  const filtered = entries.filter((p) => !new RegExp(`^"?${key}"?\\s*=`).test(p));
  filtered.push(`"${key}" = ${value}`);
  body[idx] = `flags = { ${filtered.join(', ')} }`;
  return [...lines.slice(0, start + 1), ...body, ...lines.slice(end)].join('\n');
}

/** Add a feature flag (defaults to true). */
export function addFlag(toml: string, key: string): string {
  return setFlag(toml, key, true);
}

/** Remove a feature flag from `[feature_flags]` (no-op if the section is missing). */
export function removeFlag(toml: string, key: string): string {
  const lines = toml.split('\n');
  let start = -1;
  let end = lines.length;
  for (let i = 0; i < lines.length; i++) {
    const sec = /^\[([^\]]+)\]/.exec(lines[i].trim());
    if (sec) {
      if (start !== -1 && end === lines.length) end = i;
      if (sec[1].trim() === 'feature_flags' && start === -1) start = i;
    }
  }
  if (start === -1) return toml;
  const body = lines.slice(start + 1, end);
  const updated = body.filter((l) => !new RegExp(`"?${key}"?\\s*=\\s*(true|false)`).test(l.trim()));
  return [...lines.slice(0, start + 1), ...updated, ...lines.slice(end)].join('\n');
}

/**
 * Return the list of `[section]` keys that are not declared in the
 * known structured schema (used by `ConfigForm` to surface hints).
 */
export function findUnknownSections(toml: string, knownTopLevel: Set<string>): string[] {
  const sections = new Set<string>();
  for (const line of toml.split('\n')) {
    const sec = /^\[([^\]]+)\]/.exec(line.trim());
    if (!sec) continue;
    const key = sec[1].trim();
    const top = key.split('.')[0];
    if (!knownTopLevel.has(key) && !knownTopLevel.has(top)) sections.add(key);
  }
  return Array.from(sections);
}