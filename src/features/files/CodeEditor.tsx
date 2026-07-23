/**
 * CodeEditor —— 只读代码预览 + 行号 + 可选 prism 高亮 (B9-01).
 *
 * 用 prismjs 做语法高亮(已在依赖中);不支持的扩展回退到纯文本。
 * 真实 IDE 编辑器(CodeMirror / Monaco)需要 ~500KB 包,本期不引入;
 * 只读预览已覆盖 review/diff/inspect 三个用例。
 */
import { useMemo } from 'react';
import Prism from 'prismjs';
import 'prismjs/components/prism-rust.js';
import 'prismjs/components/prism-typescript.js';
import 'prismjs/components/prism-javascript.js';
import 'prismjs/components/prism-jsx.js';
import 'prismjs/components/prism-tsx.js';
import 'prismjs/components/prism-bash.js';
import 'prismjs/components/prism-json.js';
import 'prismjs/components/prism-yaml.js';
import 'prismjs/components/prism-toml.js';
import 'prismjs/components/prism-python.js';
import 'prismjs/components/prism-go.js';
import 'prismjs/components/prism-markdown.js';
import 'prismjs/components/prism-css.js';
import s from './CodeEditor.module.css';

const EXT_LANG: Record<string, string> = {
  rs: 'rust',
  ts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  jsx: 'jsx',
  mjs: 'javascript',
  cjs: 'javascript',
  json: 'json',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  py: 'python',
  go: 'go',
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  md: 'markdown',
  css: 'css',
  scss: 'css',
  less: 'css',
  html: 'markup',
  xml: 'markup',
  svg: 'markup',
};

function detectLanguage(path: string): string {
  const i = path.lastIndexOf('.');
  if (i < 0) return 'plaintext';
  const ext = path.slice(i + 1).toLowerCase();
  return EXT_LANG[ext] ?? 'plaintext';
}

export interface CodeEditorProps {
  path: string;
  content: string;
  /** True if file was binary — render a notice instead of raw bytes. */
  binary?: boolean;
  /** True if the content was clipped at 1 MiB. */
  truncated?: boolean;
}

export function CodeEditor({ path, content, binary, truncated }: CodeEditorProps) {
  const lang = useMemo(() => detectLanguage(path), [path]);
  const highlighted = useMemo(() => {
    if (binary) return null;
    if (lang === 'plaintext' || !Prism.languages[lang]) return content;
    try {
      return Prism.highlight(content, Prism.languages[lang], lang);
    } catch {
      return content;
    }
  }, [content, lang, binary]);

  const lines = useMemo(() => content.split('\n'), [content]);

  return (
    <div className={s.root} data-lang={lang} data-testid="code-editor">
      <div className={s.header}>
        <span className={s.path}>{path}</span>
        <span className={s.lang}>{lang}</span>
        {truncated && <span className={s.truncated}>clipped to 1 MiB</span>}
      </div>
      {binary ? (
        <div className={s.binary} data-testid="code-editor-binary">
          Binary file — content not shown.
        </div>
      ) : (
        <div className={s.body}>
          <pre className={s.gutter} aria-hidden="true">
            {lines.map((_, i) => (
              <span key={i} className={s.lineNo}>{i + 1}</span>
            ))}
          </pre>
          <pre className={s.code}>
            <code
              className={`language-${lang}`}
              dangerouslySetInnerHTML={{ __html: highlighted ?? content }}
            />
          </pre>
        </div>
      )}
    </div>
  );
}