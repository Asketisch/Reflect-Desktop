/**
 * Markdown 渲染 —— react-markdown + remark-gfm + prismjs + CSS Modules。
 *
 * 代码块带复制按钮 + 深色主题。
 */
import { useState, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import Prism from 'prismjs';
import 'prismjs/components/prism-rust.js';
import 'prismjs/components/prism-typescript.js';
import 'prismjs/components/prism-bash.js';
import 'prismjs/components/prism-json.js';
import 'prismjs/components/prism-python.js';
import { Check, Copy } from 'lucide-react';
import { Icon } from '@/features/design-system';
import s from './Markdown.module.css';

interface Props {
  text: string;
}

function highlight(code: string, lang: string): string | null {
  const g = (Prism.languages as unknown as Record<string, Prism.Grammar>)[lang];
  if (!g) return null;
  return Prism.highlight(code, g, lang);
}

function escapeHtml(str: string): string {
  return str.replace(/[&<>"]/g, (c) =>
    c === '&' ? '&amp;' : c === '<' ? '&lt;' : c === '>' ? '&gt;' : '&quot;',
  );
}

export function Markdown({ text }: Props) {
  return (
    <div className={s.md}>
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
              return <code className={s.inlineCode}>{code}</code>;
            }
            return <CodeBlock code={code} lang={lang} />;
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const [copied, setCopied] = useState(false);
  const html = highlight(code, lang);

  const onCopy = useCallback(() => {
    void navigator.clipboard?.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }, [code]);

  return (
    <div className={s.codeWrap}>
      <div className={s.codeHeader}>
        <span className={s.codeLang}>{lang}</span>
        <button onClick={onCopy} className={s.copyBtn} title="Copy code">
          <Icon icon={copied ? Check : Copy} size={12} />
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre className={s.pre}>
        <code
          className={`language-${lang}`}
          dangerouslySetInnerHTML={{ __html: html ?? escapeHtml(code) }}
        />
      </pre>
    </div>
  );
}
