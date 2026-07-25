/**
 * i18n namespace —— files.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const files: Record<string, StringEntry> = {
  'files.title':                { en: 'Files',                                                                                     'zh-CN': '文件' },
  'files.subtitle':             { en: 'Browse the workspace. Click any file to preview with line numbers + syntax highlighting.',  'zh-CN': '浏览工作区。点击任意文件以预览其内容,显示行号与语法高亮。' },
  'files.open':                 { en: 'Open',                                                                                      'zh-CN': '打开' },
  'files.refresh':              { en: 'Refresh',                                                                                   'zh-CN': '刷新' },
  'files.empty':                { en: 'empty',                                                                                     'zh-CN': '空' },
  'files.noFile':               { en: 'No file selected',                                                                          'zh-CN': '未选择文件' },
  'files.noFileDesc':           { en: 'Pick a file from the tree on the left to preview its contents.',                            'zh-CN': '从左侧文件树中选择一个文件以预览其内容。' },
  'files.loadingFile':          { en: 'Loading {name}…',                                                                           'zh-CN': '正在加载 {name}…' },
  'files.search.title':         { en: 'Search',                                                                                    'zh-CN': '搜索' },
  'files.search.placeholder':   { en: 'Search in workspace…',                                                                      'zh-CN': '在工作区搜索…' },
  'files.search.resultCount':   { en: '{count} result{plural}',                                                                    'zh-CN': '{count} 个结果' },
  'files.search.empty':         { en: 'No matches',                                                                                'zh-CN': '没有匹配项' },
  'files.editor.copied':        { en: 'Copied to clipboard',                                                                       'zh-CN': '已复制到剪贴板' },
  'files.editor.readonly':      { en: 'Read-only',                                                                                 'zh-CN': '只读' },
  'files.editor.save':          { en: 'Save',                                                                                      'zh-CN': '保存' },
  'files.editor.unsaved':       { en: 'Unsaved changes',                                                                           'zh-CN': '未保存的改动' },
  'files.editor.exceedsLimit':  { en: 'File too large to edit (>{kb} KB)',                                                         'zh-CN': '文件过大,无法编辑 (>{kb} KB)' },
};

export default files;
