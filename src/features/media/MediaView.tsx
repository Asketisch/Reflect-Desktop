/**
 * MediaView —— 媒体工作室 + 电脑操控（Phase 3 条目 13）。
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
import { useI18n } from '@/utils/i18n';
import s from './MediaView.module.css';

export function MediaView() {
  const ctrl = useMediaController();
  const { t } = useI18n();
  const tabOptions = [
    { value: 'studio' as const, label: t('media.tabStudio'), hint: t('media.tabStudioHint') },
    { value: 'computer' as const, label: t('media.tabComputer'), hint: t('media.tabComputerHint') },
  ];
  return (
    <PageShell
      icon={ImageIcon}
      title={t('media.title')}
      subtitle={t('media.subtitle')}
      width="lg"
    >
      <div className={s.tabBar}>
        <SegmentedControl<MediaTab> options={tabOptions} value={ctrl.tab} onChange={ctrl.setTab} />
      </div>

      {ctrl.capabilities && (
        <div className={s.capsLine}>
          <Badge variant="neutral">{t('media.capImage', { backend: ctrl.capabilities.imageBackend })}</Badge>
          <Badge variant="neutral">{t('media.capComputer', { backend: ctrl.capabilities.computerBackend })}</Badge>
          <span className={s.capsNote}>{ctrl.capabilities.note}</span>
        </div>
      )}

      {ctrl.tab === 'studio' ? <StudioTab ctrl={ctrl} /> : <ComputerUseTab ctrl={ctrl} />}
    </PageShell>
  );
}

function StudioTab({ ctrl }: { ctrl: ReturnType<typeof useMediaController> }) {
  const { t } = useI18n();
  return (
    <div className={s.studioLayout}>
      <div className={s.studioControls}>
        <Input
          placeholder={t('media.dirPlaceholder')}
          value={ctrl.studioDir}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => ctrl.setStudioDir(e.target.value)}
          data-testid="media-dir"
        />
        <Button variant="ghost" size="sm" onClick={ctrl.refreshStudio} data-testid="media-refresh">
          <Icon icon={RefreshCw} size={12} /> {t('common.refresh')}
        </Button>
      </div>

      {ctrl.studioLoading ? (
        <div className={s.loading}><Spinner size={20} /></div>
      ) : ctrl.studioError ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={ImageIcon} />}
            title={t('media.noDirectory')}
            description={String(ctrl.studioError?.message ?? ctrl.studioError)}
          />
        </Card>
      ) : ctrl.assets.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={ImageIcon} />}
            title={t('media.noImages')}
            description={t('media.noImagesDesc')}
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
  const { t } = useI18n();
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
          <div className={s.errorTitle}>{t('media.lastActionFailed')}</div>
          <div className={s.errorBody}>{ctrl.lastError}</div>
        </Card>
      )}

      {ctrl.actionHistory.length === 0 ? (
        <Card level="flat" padding="none">
          <EmptyState
            icon={<Icon icon={MousePointer2} />}
            title={t('media.noActions')}
            description={t('media.noActionsDesc')}
          />
        </Card>
      ) : (
        <div className={s.history} data-testid="cu-history">
          <h3 className={s.subhead}>{t('media.actionHistory')}</h3>
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
  const { t } = useI18n();
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={Camera} size={14} /> {t('media.screenshot')}
      </div>
      <p className={s.cuHint}>{t('media.screenshotHint')}</p>
      <Button variant="primary" size="sm" disabled={busy} onClick={onCapture} data-testid="cu-screenshot">
        {t('media.capture')}
      </Button>
      {lastScreenshot && (
        <img
          src={lastScreenshot}
          alt={t('media.lastScreenshotAlt')}
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
  const { t } = useI18n();
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={MousePointer2} size={14} /> {t('media.click')}
      </div>
      <div className={s.coordRow}>
        <Input type="number" value={x} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setX(Number(e.target.value))} />
        <Input type="number" value={y} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setY(Number(e.target.value))} />
        <select className={s.btnSelect} value={button} onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setButton(e.target.value)}>
          <option value="left">{t('media.mouseLeft')}</option>
          <option value="right">{t('media.mouseRight')}</option>
          <option value="middle">{t('media.mouseMiddle')}</option>
        </select>
      </div>
      <Button variant="primary" size="sm" disabled={busy} onClick={() => onClick(x, y, button)} data-testid="cu-click">
        {t('media.click')}
      </Button>
    </Card>
  );
}

function MoveAction({ onClick, busy }: { onClick: (x: number, y: number) => void; busy: boolean }) {
  const [x, setX] = useState(0);
  const [y, setY] = useState(0);
  const { t } = useI18n();
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={MousePointer2} size={14} /> {t('media.move')}
      </div>
      <div className={s.coordRow}>
        <Input type="number" value={x} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setX(Number(e.target.value))} />
        <Input type="number" value={y} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setY(Number(e.target.value))} />
      </div>
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(x, y)} data-testid="cu-move">
        {t('media.move')}
      </Button>
    </Card>
  );
}

function ScrollAction({ onClick, busy }: { onClick: (dx: number, dy: number) => void; busy: boolean }) {
  const [dx, setDx] = useState(0);
  const [dy, setDy] = useState(-3);
  const { t } = useI18n();
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>{t('media.scroll')}</div>
      <div className={s.coordRow}>
        <Input type="number" value={dx} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDx(Number(e.target.value))} />
        <Input type="number" value={dy} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDy(Number(e.target.value))} />
      </div>
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(dx, dy)} data-testid="cu-scroll">
        {t('media.scroll')}
      </Button>
    </Card>
  );
}

function KeyTypeAction({ onClick, busy }: { onClick: (text: string) => void; busy: boolean }) {
  const [text, setText] = useState('hello');
  const { t } = useI18n();
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={Keyboard} size={14} /> {t('media.typeText')}
      </div>
      <Input value={text} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setText(e.target.value)} />
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(text)} data-testid="cu-type">
        {t('media.type')}
      </Button>
    </Card>
  );
}

function KeyComboAction({ onClick, busy }: { onClick: (keys: string) => void; busy: boolean }) {
  const [keys, setKeys] = useState('ctrl+c');
  const { t } = useI18n();
  return (
    <Card level="outlined" padding="md" className={s.cuCard}>
      <div className={s.cuTitle}>
        <Icon icon={Keyboard} size={14} /> {t('media.keyCombo')}
      </div>
      <Input value={keys} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setKeys(e.target.value)} placeholder={t('media.keyComboPlaceholder')} />
      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onClick(keys)} data-testid="cu-combo">
        {t('media.combo')}
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
