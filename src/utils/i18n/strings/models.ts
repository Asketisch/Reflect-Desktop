/**
 * i18n 命名空间 —— models.*
 *
 * 由 strings/index.ts merge 进 STRINGS dict；不要直接 import 此模块 ——
 * 走 `@/utils/i18n` 的统一入口。
 */
import type { StringEntry } from '../types';

const models: Record<string, StringEntry> = {
  'models.title':              { en: 'Models',                                                   'zh-CN': '模型' },
  'models.subtitle':           { en: 'Available models across active providers.',                'zh-CN': '当前提供商可用的模型。' },
  'models.empty':              { en: 'No models available — check provider config.',             'zh-CN': '没有可用模型——请检查提供商配置。' },
  'models.context':            { en: 'Context window: {tokens} tokens',                          'zh-CN': '上下文窗口: {tokens} tokens' },
  'models.pricing':            { en: 'Pricing: in {in} / out {out} (micro-USD/Mtok)',            'zh-CN': '价格: 输入 {in} / 输出 {out} (micro-USD/Mtok)' },
  'models.currentModel':       { en: 'Current model',                                            'zh-CN': '当前模型' },
  'models.effort':             { en: 'Reasoning effort',                                         'zh-CN': '推理 effort' },
  'models.effortHint':         { en: 'Switch effort via Op::SetEffort for the active session.',  'zh-CN': '通过 Op::SetEffort 切换当前会话的 effort。' },
  'models.applied':            { en: 'Applied ✓',                                                'zh-CN': '已应用 ✓' },
  'models.applyEffort':        { en: 'Apply effort',                                             'zh-CN': '应用 effort' },
  'models.effort.lowDesc':     { en: 'Fastest · minimal reasoning',                              'zh-CN': '最快 · 最少推理' },
  'models.effort.mediumDesc':  { en: 'Balanced (recommended)',                                   'zh-CN': '平衡 (推荐)' },
  'models.effort.highDesc':    { en: 'Deep · slowest',                                           'zh-CN': '深度推理 · 最慢' },
  'models.noProvider':         { en: 'No provider configured',                                   'zh-CN': '未配置 provider' },
  'models.noProviderDesc':     { en: 'Set an API key to enable a model.',                        'zh-CN': '设置 API 密钥以启用模型。' },
  'models.configureHint':      { en: 'Configure one in Settings to get started.',                'zh-CN': '请在设置中配置。' },
  'models.settingsHint':       { en: 'Model spec is edited in Settings → {path}.',               'zh-CN': '模型规范在 Settings → {path} 中编辑。' },
};

export default models;
