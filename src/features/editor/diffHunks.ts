/**
 * unified diff 解析与 inverse-patch。
 *
 * agent 的 edit/write 工具以 `ContentBlock::Diff { unified_diff }` 上报
 * before→after 变更;本模块把它解析成 hunk 列表供 CodeMirror 装饰
 * (新增行绿色背景、删除行红色块),并支持 Cursor 式逐块拒绝——
 * 通过内容锚定的 inverse-apply 把 hunk 还原回原文。
 *
 * 全部为纯函数,不触碰 IPC。
 */

export interface DiffHunk {
  /** @@ -oldStart,oldLines +newStart,newLines @@ 中的旧文件起始行(1 基)。 */
  oldStart: number;
  /** 新文件起始行(1 基)。 */
  newStart: number;
  /** hunk 内被删除的行(原文件内容,不含 '-' 前缀)。 */
  removed: string[];
  /** hunk 内新增的行(新文件内容,不含 '+' 前缀)。 */
  added: string[];
  /** hunk 头部的上下文行(取自新文件,用于显示与锚定)。 */
  contextBefore: string[];
  contextAfter: string[];
}

/**
 * 解析 unified diff 文本。只取行级 hunk;`No newline` 标记、文件头
 * (---/+++)与无法识别的行忽略。没有任何 hunk 时返回空数组。
 */
export function parseUnifiedDiff(diff: string): DiffHunk[] {
  const hunks: DiffHunk[] = [];
  const lines = diff.split('\n');
  let current: DiffHunk | null = null;

  for (const line of lines) {
    const header = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (header) {
      current = {
        oldStart: Number(header[1]),
        newStart: Number(header[2]),
        removed: [],
        added: [],
        contextBefore: [],
        contextAfter: [],
      };
      hunks.push(current);
      continue;
    }
    if (!current) continue;
    // 多文件 diff 的文件头(`--- a/x` / `+++ b/x`)以 -/+ 开头,若不显式
    // 跳过会被吞进当前 hunk 污染锚定(单文件 ContentBlock::Diff 不触发,
    // 防御粘接/拼接产生的多段 diff)。
    if (line.startsWith('--- ') || line.startsWith('+++ ')) continue;
    if (line.startsWith('+')) {
      current.added.push(line.slice(1));
    } else if (line.startsWith('-')) {
      current.removed.push(line.slice(1));
    } else if (line.startsWith('\\')) {
      // "\ No newline at end of file" — 忽略。
    } else if (line.startsWith(' ') || line === '') {
      // 上下文行(diff 末尾的空行归入上下文;分隔空行会因不在任何 hunk 而跳过)。
      if (current.added.length > 0 || current.removed.length > 0) {
        current.contextAfter.push(line.startsWith(' ') ? line.slice(1) : '');
      } else {
        current.contextBefore.push(line.startsWith(' ') ? line.slice(1) : '');
      }
    }
  }
  // 尾部空上下文行多半是 diff 字符串结尾的换行产物,裁掉。
  for (const h of hunks) {
    while (h.contextAfter.length > 0 && h.contextAfter[h.contextAfter.length - 1] === '') {
      h.contextAfter.pop();
    }
  }
  return hunks;
}

/**
 * 在 `content` 中锚定并反转单个 hunk:定位「contextBefore + added +
 * contextAfter」序列,替换为「contextBefore + removed + contextAfter」。
 *
 * 用内容锚定而非行号——agent 连续多次编辑同一文件时,前序 hunk 的拒绝
 * 会使行号漂移;行首空白完全一致的锚点也可能歧义,故只要求序列唯一。
 * 找不到唯一锚点 → null(调用方提示用户手动处理)。
 */
export function applyInverseHunk(content: string, hunk: DiffHunk): string | null {
  const contentLines = splitKeepTrailing(content);

  // 纯删除 hunk:新内容里没有 added 锚点。优先用 hunk 的上下文行做内容
  // 锚定——oldStart 是「该次编辑前」的行号,若用户先拒绝了更早的 hunk
  // (行号整体漂移),按行号插回会把删除行放错位置;上下文行仍在当前内容
  // 中时,锚定与 added hunk 一样可靠。
  if (hunk.added.length === 0) {
    if (hunk.removed.length === 0) return null;
    if (hunk.contextAfter.length > 0) {
      const positions = findAll(contentLines, hunk.contextAfter);
      if (positions.length === 1) {
        const at = positions[0];
        const next = [...contentLines.slice(0, at), ...hunk.removed, ...contentLines.slice(at)];
        return joinTrimmingTrailing(next, content);
      }
    }
    if (hunk.contextBefore.length > 0) {
      const positions = findAll(contentLines, hunk.contextBefore);
      if (positions.length === 1) {
        const at = positions[0] + hunk.contextBefore.length;
        const next = [...contentLines.slice(0, at), ...hunk.removed, ...contentLines.slice(at)];
        return joinTrimmingTrailing(next, content);
      }
    }
    // 锚定失败退化为行号(旧行为);唯一性已不可证,尽量插回。
    const insertAt = Math.min(Math.max(hunk.oldStart - 1, 0), contentLines.length);
    const next = [...contentLines.slice(0, insertAt), ...hunk.removed, ...contentLines.slice(insertAt)];
    return joinTrimmingTrailing(next, content);
  }

  const target = [...hunk.contextBefore, ...hunk.added, ...hunk.contextAfter];
  const replacement = [...hunk.contextBefore, ...hunk.removed, ...hunk.contextAfter];

  const positions = findAll(contentLines, target);
  if (positions.length !== 1) return null;

  const start = positions[0];
  const next = [...contentLines.slice(0, start), ...replacement, ...contentLines.slice(start + target.length)];
  return joinTrimmingTrailing(next, content);
}

/**
 * 反转全部 hunk(Reject All):按顺序逐个应用;任一块锚定失败 → null,
 * 保证不落盘半恢复状态。
 */
export function applyInverseAll(content: string, hunks: DiffHunk[]): string | null {
  let current = content;
  for (const hunk of [...hunks].reverse()) {
    const next = applyInverseHunk(current, hunk);
    if (next === null) return null;
    current = next;
  }
  return current;
}

/** 按行拆分;保留结尾换行语义(以末尾是否空元素近似)。 */
function splitKeepTrailing(content: string): string[] {
  if (content === '') return [''];
  const lines = content.split('\n');
  return lines;
}

/** 与 splitKeepTrailing 配对:原内容不带尾换行时,去掉拆分引入的尾空元素。 */
function joinTrimmingTrailing(lines: string[], original: string): string {
  if (original !== '' && !original.endsWith('\n') && lines.length > 0 && lines[lines.length - 1] === '') {
    const copy = [...lines];
    while (copy.length > 0 && copy[copy.length - 1] === '') copy.pop();
    return copy.join('\n');
  }
  return lines.join('\n');
}

/** 在行数组中查找 target 序列的所有起始下标。 */
function findAll(haystack: string[], target: string[]): number[] {
  if (target.length === 0) return [];
  const out: number[] = [];
  for (let i = 0; i + target.length <= haystack.length; i++) {
    let matched = true;
    for (let j = 0; j < target.length; j++) {
      if (haystack[i + j] !== target[j]) {
        matched = false;
        break;
      }
    }
    if (matched) out.push(i);
  }
  return out;
}

/** hunk 是否为纯新增(删除行为空,拒绝 = 直接删除这些行)。 */
export function isPureAddition(hunk: DiffHunk): boolean {
  return hunk.removed.length === 0;
}

/**
 * 在当前内容里定位 hunk 的新增行区间(供编辑器装饰)。
 * 返回 [addedStartLine, addedEndLineExcl) 的 0 基行号;纯删除 hunk 返回
 * 插入点(单行);锚定失败返回 null。与 applyInverseHunk 同样内容锚定。
 */
export function locateHunk(
  content: string,
  hunk: DiffHunk,
): { addedStart: number; addedEnd: number } | null {
  const lines = content.split('\n');
  if (hunk.added.length === 0) {
    const at = Math.min(Math.max(hunk.newStart - 1, 0), lines.length);
    return { addedStart: at, addedEnd: at };
  }
  const target = [...hunk.contextBefore, ...hunk.added];
  const positions = findAll(lines, target);
  if (positions.length !== 1) return null;
  const addedStart = positions[0] + hunk.contextBefore.length;
  return { addedStart, addedEnd: addedStart + hunk.added.length };
}
