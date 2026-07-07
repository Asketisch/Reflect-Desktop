/**
 * M1.4 简易 Markdown 渲染 —— `react-markdown` + `remark-gfm` + `prismjs` 高亮。
 *
 * M2.x 升级到 streamdown；M1 接受 react-markdown 的非打字机体验。
 */
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import Prism from 'prismjs';
import 'prismjs/components/prism-rust.js';
import 'prismjs/components/prism-typescript.js';
import 'prismjs/components/prism-bash.js';
import 'prismjs/components/prism-json.js';
import 'prismjs/components/prism-python.js';

interface Props {
  text: string;
}

function highlight(code: string, lang: string): string | null {
  const g = (Prism.languages as unknown as Record<string, Prism.Grammar>)[lang];
  if (!g) return null;
  return Prism.highlight(code, g, lang);
}

export function Markdown({ text }: Props) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code({ inline, className, children }: {
          inline?: boolean;
          className?: string;
          children?: React.ReactNode;
        }) {
          const code = String(children ?? '').replace(/\n$/, '');
          const lang = /language-(\w+)/.exec(className ?? '')?.[1] ?? '';
          if (inline || !lang) {
            return <code style={inlineStyle}>{code}</code>;
          }
          const html = highlight(code, lang);
          return (
            <pre style={preStyle}>
              <code
                className={`language-${lang}`}
                dangerouslySetInnerHTML={{ __html: html ?? escapeHtml(code) }}
              />
            </pre>
          );
        },
      }}
    >
      {text}
    </ReactMarkdown>
  );
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"]/g, (c) =>
    c === '&' ? '&amp;' : c === '<' ? '&lt;' : c === '>' ? '&gt;' : '&quot;',
  );
}

const inlineStyle: React.CSSProperties = {
  background: '#f1f5f9',
  padding: '2px 4px',
  borderRadius: 3,
  fontFamily: 'ui-monospace, monospace',
  fontSize: '0.9em',
};

const preStyle: React.CSSProperties = {
  position: 'relative',
  background: '#0f172a',
  color: '#e2e8f0',
  padding: 12,
  borderRadius: 6,
  overflow: 'auto',
  fontFamily: 'ui-monospace, monospace',
  fontSize: 13,
};
