/**
 * 助手正文分段 —— 把混进正文的"思考过程"和运行时标记拆出来。
 *
 * 两类不该按正文渲染的内容:
 * - `<think>…</think>` 推理:MiniMax M3 / DeepSeek 系推理模型把推理
 *   以标签形式混在正文流式输出里(而非 Anthropic thinking 块 /
 *   ThinkingDelta 事件),后端原样透传;
 * - `FINAL ANSWER:` 标记:runtime 的 nudge / auto-continue 提醒注入
 *   `reflect_prompt::FINAL_ANSWER_TEMPLATE`,模型会照字面回显到答案里。
 *
 * 在渲染层拆分(而非 reducer 层),streaming 增量、agent_message 定稿、
 * 会话回放三条路径统一生效;正文数据保持原样,不丢信息。
 */

export interface AssistantSegments {
  /** 思考过程文本(无则为 null)。 */
  thinking: string | null;
  /** 去除思考块与 FINAL ANSWER 标记后的可见正文。 */
  content: string;
}

const THINK_OPEN = '<think>';
const THINK_CLOSE = '</think>';

/** 行首标记(容忍 `> ` / `**` / 列表符等 markdown 修饰);只删标记本身,保留行内余文。 */
const FINAL_ANSWER_MARKER = /^[ \t>*-]*\*{0,2}FINAL ANSWER\*{0,2}[:：][ \t]*/gm;
/** 模型照字面回显模板时,<answer> 占位符会单独成行 —— 一并移除。 */
const TEMPLATE_PLACEHOLDER_LINE = /^[ \t>*-]*<answer>[ \t]*$/gm;

export function splitAssistantText(raw: string): AssistantSegments {
  let thinking: string | null = null;
  let content = raw;

  const openIdx = content.indexOf(THINK_OPEN);
  if (openIdx !== -1) {
    const before = content.slice(0, openIdx);
    const rest = content.slice(openIdx + THINK_OPEN.length);
    const closeIdx = rest.indexOf(THINK_CLOSE);
    if (closeIdx !== -1) {
      thinking = stripRuntimeMarkers(`${before}${rest.slice(0, closeIdx)}`).trim() || null;
      content = rest.slice(closeIdx + THINK_CLOSE.length);
    } else {
      // 流式中标签未闭合:目前全部是推理,正文为空。
      thinking = stripRuntimeMarkers(`${before}${rest}`).trim() || null;
      content = '';
    }
  }

  return { thinking, content: stripRuntimeMarkers(content).replace(/^\s+/, '') };
}

function stripRuntimeMarkers(text: string): string {
  return text
    .replace(FINAL_ANSWER_MARKER, '')
    .replace(TEMPLATE_PLACEHOLDER_LINE, '')
    .replace(/\n{3,}/g, '\n\n');
}
