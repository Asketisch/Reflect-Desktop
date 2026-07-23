/**
 * fuzzy —— 轻量级模糊匹配(B10-01)。
 *
 * 给定 query 与 list,返回 [(item, score)] 按 score 降序;
 * score < 0 表示未匹配。规则:
 *   - 子序列匹配(字符按顺序出现),case-insensitive
 *   - 命中开头得高分,连续匹配得更高分
 *   - 完全相等得最高分
 */
export interface FuzzyHit<T> {
  item: T;
  score: number;
  /** Indices into the source text that matched, for highlighting. */
  matches: number[];
}

export function fuzzy<T extends { label: string; keywords?: string[]; hint?: string }>(
  query: string,
  list: T[],
): FuzzyHit<T>[] {
  const q = query.trim().toLowerCase();
  if (!q) return list.map((item) => ({ item, score: 0, matches: [] }));
  const hits: FuzzyHit<T>[] = [];
  for (const item of list) {
    const haystacks: { text: string; boost: number }[] = [
      { text: item.label, boost: 2 },
      ...(item.keywords ?? []).map((k) => ({ text: k, boost: 1.5 })),
      ...(item.hint ? [{ text: item.hint, boost: 0.5 }] : []),
    ];
    let bestScore = -1;
    let bestMatches: number[] = [];
    for (const { text, boost } of haystacks) {
      const r = scoreOne(q, text.toLowerCase());
      if (r.score < 0) continue;
      const s = r.score * boost;
      if (s > bestScore) {
        bestScore = s;
        bestMatches = r.matches;
      }
    }
    if (bestScore >= 0) {
      hits.push({ item, score: bestScore, matches: bestMatches });
    }
  }
  return hits.sort((a, b) => b.score - a.score);
}

function scoreOne(q: string, text: string): { score: number; matches: number[] } {
  if (text === q) return { score: 100, matches: text.split('').map((_, i) => i) };
  if (text.startsWith(q)) {
    return { score: 50, matches: Array.from({ length: q.length }, (_, i) => i) };
  }
  // subsequence
  const matches: number[] = [];
  let qi = 0;
  let run = 0;
  let bestRun = 0;
  let score = 0;
  for (let i = 0; i < text.length && qi < q.length; i++) {
    if (text[i] === q[qi]) {
      matches.push(i);
      run += 1;
      bestRun = Math.max(bestRun, run);
      score += 1;
      if (run > 1) score += 2 * (run - 1);
      qi += 1;
    } else {
      run = 0;
    }
  }
  if (qi < q.length) return { score: -1, matches: [] };
  // bonus for compact matches
  if (bestRun === q.length) score += 8;
  if (matches.length > 0 && matches[matches.length - 1] - matches[0] === q.length - 1) score += 4;
  return { score, matches };
}