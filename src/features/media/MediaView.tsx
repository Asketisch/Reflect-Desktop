/**
 * MediaView —— Media Studio + Computer Use (Phase 3 item 13).
 *
 * 双 tab:
 * - **Studio**:目录扫描图片列表。backend = `RealImageBackend`(`image` crate),
 *   返回完整元数据(含像素维度)。
 * - **Computer Use**:鼠标 / 键盘控制面板。backend = `RealComputerBackend`
 *   (`xcap` 截屏 + `enigo` 鼠标键盘)。截图走独立 `reflect_screenshot` 命令
 *   (返回 base64 PNG),其余动作走 `reflect_computer_use`。
 *
 * 注:CI / headless 环境下 xcap/enigo 会返回 `MediaError::Unavailable`,
 * 错误以 toast + 错误卡片呈现,不 panic。
 */
import { useState } from 'react';
import { Image as ImageIcon, MousePointer2, Camera, Keyboard, RefreshCw } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import {
  Card,
  Icon,
  EmptyState,
  Spinner,
  Input,
  Button,
  Badge,
  SegmentedControl,
} from '@/features/design-system';
import { useMediaController, type MediaTab } from './useMediaController';
import s from './MediaView.module.css';

const TAB_OPTIONS = [
  { value: 'studio' as const, label: 'Studio', hint: 'Image assets' },
  { value: 'computer' as const, label: 'Computer Use', hint: 'Mouse + keyboard' },
];

export function MediaView() {
  const ctrl = useMediaController();
  return (
    <PageShell
      icon={ImageIcon}
      title="Media Studio"
      subtitle="Local image assets + computer-use controls."
      width="lg"
    >
      <div className={s.tabBar}>
        <SegmentedControl<MediaTab> options={TAB_OPTIONS} value={ctrl.tab} onChange={ctrl.setTab} />
      </div>

      {ctrl.capabilities && (
        <div className={s.capsLine}>
          <Badge variant="neutral">image: {ctrl.capabilities.imageBackend}</Badge>
          <Badge variant="neutral">computer: {ctrl.capabilities.computerBackend}</Badge>
          <span className={s.capsNote}>{ctrl.capabilities.note}</span>
        </div>
      )}

      {ctrl.tab === 'studio' ? <StudioTab ctrl={ctrl} /> : <ComputerUseTab ctrl={ctrl} />}
    </PageShell>
  );
}

function StudioTab({ ctrl }: { ctrl: ReturnType<typeof useMediaController> }) {
  return (
    <div className={s.studioLayout}>
      <div className={s.studioControls}>
        <Input
          placeholder="Directory path (e.g. /Users/me/Pictures)"
          value={ctrl.studioDir}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => ctrl.setStudioDir(e.target.value)}
          data-testid="media-dir"
        />
        <Button variant="ghost" size="sm" onClick={ctrl.refreshStudio} data-testid="media-refresh">
          <Icon icon={RefreshCw} size={12} /> Refresh
        </Button>
      </div>

      {ctrl.studioLoading ? (
        <div className={s.loading}><Spinner size={20} /></div>
      ) : ctrl.studioError ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={ImageIcon} />}
            title="No directory / cannot read"
            description={String(ctrl.studioError?.message ?? ctrl.studioError)}
          />
        </Card>
      ) : ctrl.assets.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={ImageIcon} />}
            title="No images yet"
            description="Point the directory input to a folder containing PNG / JPEG / GIF / WebP / BMP files."
          />
        </Card>
      ) : (
        <div className={s.assetGrid} data-testid="media-grid">
          {ctrl.assets.map((a) => (
            <Card key={a.path} level="outlined" padding="sm">
              <div className={s.asset}>
                <div className={s.assetIcon}>
                  <Icon icon={ImageIcon} size={24} />
                </div>
                <div className={s.assetMeta}>
                  <div className={s.assetName} title={a.path}>{a.filename}</div>
                  <div className={s.assetSub}>
                    {formatBytes(a.sizeBytes)} · {a.mimeType ?? '?'}
                    {a.width && a.height && <> · {a.width}×{a.height}</>}
                  </div>
                  <div className={s.assetPath}>{new Date(a.modifiedAtMs).toLocaleString()}</div>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

function ComputerUseTab({ ctrl }: { ctrl: ReturnType<typeof useMediaController> }) {
  return (
    <div className={s.cuLayout}>
      <div className={s.cuControls}>
        <ScreenshotAction
          onCapture={() => void ctrl.captureScreenshot()}
          busy={ctrl.busy}
          lastScreenshot={ctrl.lastScreenshot}
        />
        <ClickAction onClick={(x, y, button) => void ctrl.executeAction({ kind: 'mouseClick', params: { x, y, button } })} busy={ctrl.busy} />
        <MoveAction onClick={(x, y) => void ctrl.executeAction({ kind: 'mouseMove', params: { x, y } })} busy={ctrl.busy} />
        <ScrollAction onClick={(dx, dy) => void ctrl.executeAction({ kind: 'scroll', params: { dx, dy } })} busy={ctrl.busy} />
        <KeyTypeAction onClick={(text) => void ctrl.executeAction({ kind: 'keyType', params: { text } })} busy={ctrl.busy} />
        <KeyComboAction onClick={(keys) => void ctrl.executeAction({ kind: 'keyCombo', params: { keys } })} busy={ctrl.busy} />
      </div>

      {ctrl.lastError && (
        <Card level="outlined" padding="md" className={s.errorCard} data-level="error">
          <div className={s.errorTitle}>Last action failed</div>
          <div className={s.errorBody}>{ctrl.lastError}</div>
        </Card>
      )}

      {ctrl.actionHistory.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={MousePointer2} />}
            title="No actions yet"
            description="Click any control above to drive the mouse, keyboard, or capture a screenshot. On macOS the first action may prompt for Accessibility / Screen Recording permission."
          />
        </Card>
      ) : (
        <div className={s.history} data-testid="cu-history">
          <h3 className={s.subhead}>Action history (latest 20)</h3>
          {ctrl.actionHistory.map((a, i) => (
            <Card key={i} level="flat" padding="sm" className={s.histRow}>
              <code>{summarize(a)}</code>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

function ScreenshotAction({
  onCapture,
  busy,
  lastScreenshot,
}: {
  onCapture: () => void;
  busy: boolean;
  lastScreenshot: string | null;
}) {
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={Camera} size={14} /> Screenshot
      </div>
      <p className={s.cuHint}>Capture the full screen (PNG).</p>
      <Button variant="primary" size="sm" disabled={busy} onClick={onCapture} data-testid="cu-screenshot">
        Capture
      </Button>
      {lastScreenshot && (
        <img
          src={lastScreenshot}
          alt="Last screenshot"
          className={s.shotPreview}
          data-testid="cu-screenshot-preview"
        />
      )}
    </Card>
  );
}

function ClickAction({
  onClick,
  busy,
}: {
  onClick: (x: number, y: number, button: string) => void;
  busy: boolean;
}) {
  const [x, setX] = useState(0);
  const [y, setY] = useState(0);
  const [button, setButton] = useState('left');
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={MousePointer2} size={14} /> Click
      </div>
      <div className={s.coordRow}>
        <Input type="number" value={x} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setX(Number(e.target.value))} />
        <Input type="number" value={y} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setY(Number(e.target.value))} />
        <select className={s.btnSelect} value={button} onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setButton(e.target.value)}>
          <option value="left">left</option>
          <option value="right">right</option>
          <option value="middle">middle</option>
        </select>
      </div>
      <Button variant="primary" size="sm" disabled={busy} onClick={() => onClick(x, y, button)} data-testid="cu-click">
        Click
      </Button>
    </Card>
  );
}

function MoveAction({ onClick, busy }: { onClick: (x: number, y: number) => void; busy: boolean }) {
  const [x, setX] = useState(0);
  const [y, setY] = useState(0);
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={MousePointer2} size={14} /> Move
      </div>
      <div className={s.coordRow}>
        <Input type="number" value={x} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setX(Number(e.target.value))} />
        <Input type="number" value={y} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setY(Number(e.target.value))} />
      </div>
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(x, y)} data-testid="cu-move">
        Move
      </Button>
    </Card>
  );
}

function ScrollAction({ onClick, busy }: { onClick: (dx: number, dy: number) => void; busy: boolean }) {
  const [dx, setDx] = useState(0);
  const [dy, setDy] = useState(-3);
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>Scroll</div>
      <div className={s.coordRow}>
        <Input type="number" value={dx} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDx(Number(e.target.value))} />
        <Input type="number" value={dy} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDy(Number(e.target.value))} />
      </div>
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(dx, dy)} data-testid="cu-scroll">
        Scroll
      </Button>
    </Card>
  );
}

function KeyTypeAction({ onClick, busy }: { onClick: (text: string) => void; busy: boolean }) {
  const [text, setText] = useState('hello');
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={Keyboard} size={14} /> Type text
      </div>
      <Input value={text} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setText(e.target.value)} />
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(text)} data-testid="cu-type">
        Type
      </Button>
    </Card>
  );
}

function KeyComboAction({ onClick, busy }: { onClick: (keys: string) => void; busy: boolean }) {
  const [keys, setKeys] = useState('ctrl+c');
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={Keyboard} size={14} /> Key combo
      </div>
      <Input value={keys} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setKeys(e.target.value)} placeholder="e.g. cmd+shift+p" />
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(keys)} data-testid="cu-combo">
        Combo
      </Button>
    </Card>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function summarize(action: { kind: string; params?: Record<string, unknown> | null }): string {
  const p = action.params ?? {};
  switch (action.kind) {
    case 'screenshot':
      return 'screenshot';
    case 'mouseMove':
      return `mouseMove (${p.x}, ${p.y})`;
    case 'mouseClick':
      return `mouseClick (${p.x}, ${p.y}, ${p.button ?? 'left'})`;
    case 'keyType':
      return `keyType "${String(p.text).slice(0, 20)}"`;
    case 'keyCombo':
      return `keyCombo ${p.keys}`;
    case 'scroll':
      return `scroll (${p.dx}, ${p.dy})`;
    default:
      return action.kind;
  }
}
