/**
 * i18n 命名空间 - media.*
 */
import type { StringEntry } from '../types';

const media: Record<string, StringEntry> = {
  'media.title':              { en: 'Media Studio', 'zh-CN': '媒体工作室' },
  'media.subtitle':           { en: 'Local image assets + computer-use controls.', 'zh-CN': '本地图片素材 + 电脑操控。' },
  'media.tabStudio':          { en: 'Studio', 'zh-CN': '工作室' },
  'media.tabStudioHint':      { en: 'Image assets', 'zh-CN': '图片素材' },
  'media.tabComputer':        { en: 'Computer Use', 'zh-CN': '电脑操控' },
  'media.tabComputerHint':    { en: 'Mouse + keyboard', 'zh-CN': '鼠标 + 键盘' },
  'media.capImage':           { en: 'image: {backend}', 'zh-CN': 'image：{backend}' },
  'media.capComputer':        { en: 'computer: {backend}', 'zh-CN': 'computer：{backend}' },
  'media.dirPlaceholder':     { en: 'Directory path (e.g. /Users/me/Pictures)', 'zh-CN': '目录路径（如 /Users/me/Pictures）' },
  'media.noDirectory':        { en: 'No directory / cannot read', 'zh-CN': '无目录 / 无法读取' },
  'media.noImages':           { en: 'No images yet', 'zh-CN': '暂无图片' },
  'media.noImagesDesc':       { en: 'Point the directory input to a folder containing PNG / JPEG / GIF / WebP / BMP files.', 'zh-CN': '将目录输入框指向包含 PNG / JPEG / GIF / WebP / BMP 文件的文件夹。' },
  'media.lastActionFailed':   { en: 'Last action failed', 'zh-CN': '上一次操作失败' },
  'media.noActions':          { en: 'No actions yet', 'zh-CN': '暂无操作' },
  'media.noActionsDesc':      { en: 'Click any control above to drive the mouse, keyboard, or capture a screenshot. On macOS the first action may prompt for Accessibility / Screen Recording permission.', 'zh-CN': '点击上方任一控件来操控鼠标、键盘或截取屏幕。在 macOS 上，首次操作可能会请求“辅助功能 / 屏幕录制”权限。' },
  'media.actionHistory':      { en: 'Action history (latest 20)', 'zh-CN': '操作历史（最近 20 条）' },
  'media.screenshot':         { en: 'Screenshot', 'zh-CN': '屏幕截图' },
  'media.screenshotHint':     { en: 'Capture the full screen (PNG).', 'zh-CN': '截取整个屏幕（PNG）。' },
  'media.capture':            { en: 'Capture', 'zh-CN': '截取' },
  'media.lastScreenshotAlt':  { en: 'Last screenshot', 'zh-CN': '上一次截图' },
  'media.click':              { en: 'Click', 'zh-CN': '点击' },
  'media.mouseLeft':          { en: 'left', 'zh-CN': '左键' },
  'media.mouseRight':         { en: 'right', 'zh-CN': '右键' },
  'media.mouseMiddle':        { en: 'middle', 'zh-CN': '中键' },
  'media.move':               { en: 'Move', 'zh-CN': '移动' },
  'media.scroll':             { en: 'Scroll', 'zh-CN': '滚动' },
  'media.typeText':           { en: 'Type text', 'zh-CN': '输入文本' },
  'media.type':               { en: 'Type', 'zh-CN': '输入' },
  'media.keyCombo':           { en: 'Key combo', 'zh-CN': '组合键' },
  'media.combo':              { en: 'Combo', 'zh-CN': '组合' },
  'media.keyComboPlaceholder': { en: 'e.g. cmd+shift+p', 'zh-CN': '如 cmd+shift+p' },
  'media.toastExecuted':      { en: 'executed: {action}', 'zh-CN': '已执行：{action}' },
  'media.toastScreenshot':    { en: 'Screenshot captured', 'zh-CN': '截图完成' },
} as const;

export default media;
