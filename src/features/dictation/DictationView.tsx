/**
 * Dictation —— 语音输入页面。
 *
 * 使用 Web Speech API（webkitSpeechRecognition / SpeechRecognition）。
 * Tauri macOS/Windows 上的 WebView 支持该 API；不支持时显示降级提示。
 *
 * - 提供"按下开始 / 松开结束 / 复制到剪贴板" UI。
 * - 显示识别中间态 + 错误信息。
 * - 提供"插入到 composer"回调入口，由上层 AppShell 注入。
 */
import { useEffect, useRef, useState } from 'react';
import { Mic, MicOff, Copy, ClipboardPaste, Trash2, AlertTriangle } from 'lucide-react';
import { PageShell } from '@/features/shell/PageShell';
import { Card, Badge, Icon, Button } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { useDictation } from './useDictation';
import s from './DictationView.module.css';

export interface DictationViewProps {
  /** 把 transcript 注入到当前 composer 输入。 */
  onInsertIntoComposer?: (text: string) => void;
}

export function DictationView({ onInsertIntoComposer }: DictationViewProps) {
  const { t, dictationLang } = useI18n();
  const { isSupported, isListening, transcript, error, start, stop, reset } =
    useDictation({ lang: dictationLang, continuous: true });

  const [holdMode, setHoldMode] = useState(false);
  const copyOkRef = useRef<number | null>(null);
  const [copyOk, setCopyOk] = useState(false);

  async function copy() {
    if (!transcript) return;
    try {
      await navigator.clipboard.writeText(transcript);
      setCopyOk(true);
      if (copyOkRef.current) window.clearTimeout(copyOkRef.current);
      copyOkRef.current = window.setTimeout(() => setCopyOk(false), 1500);
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    return () => {
      if (copyOkRef.current) window.clearTimeout(copyOkRef.current);
    };
  }, []);

  return (
    <PageShell
      icon={Mic}
      title={t('dictation.title')}
      subtitle={
        <>
          {t('dictation.subtitle')} <Badge variant="info">{t('dictation.live')}</Badge>
        </>
      }
      width="md"
    >
      <Card level="flat" padding="lg" className={s.card}>
        {!isSupported ? (
          <div className={s.empty}>
            <Icon icon={MicOff} size={32} />
            <h3>{t('dictation.unavailable')}</h3>
            <p>
              {t('dictation.unavailableDesc', { code: 'SpeechRecognition' })}
            </p>
            <Badge variant="warning">{t('dictation.unavailableBadge')}</Badge>
          </div>
        ) : (
          <>
            <div className={s.row}>
              <Button
                variant={isListening ? 'danger' : 'primary'}
                onClick={() => (isListening ? stop() : start())}
                aria-pressed={isListening}
                aria-label={isListening ? t('dictation.stopAria') : t('dictation.startAria')}
              >
                <Icon icon={isListening ? MicOff : Mic} size={16} />
                {isListening ? t('dictation.stop') : t('dictation.start')}
              </Button>

              <label className={s.checkbox}>
                <input
                  type="checkbox"
                  checked={holdMode}
                  onChange={(e) => setHoldMode(e.target.checked)}
                />
                <span>{t('dictation.holdLabel')}</span>
              </label>
            </div>

            {holdMode && (
              <div
                className={s.holdArea}
                onMouseDown={() => start()}
                onMouseUp={() => stop()}
                onMouseLeave={() => isListening && stop()}
                onTouchStart={() => start()}
                onTouchEnd={() => stop()}
                role="button"
                tabIndex={0}
                aria-label={t('dictation.holdAria')}
              >
                {isListening ? t('dictation.holdActive') : t('dictation.holdIdle')}
              </div>
            )}

            <textarea
              className={s.transcript}
              rows={8}
              value={transcript}
              placeholder={isListening ? t('dictation.placeholderActive') : t('dictation.placeholderIdle')}
              readOnly
              aria-label={t('dictation.transcriptAria')}
            />

            {error && (
              <div className={s.error} role="alert">
                <Icon icon={AlertTriangle} size={16} />
                <span>{error}</span>
              </div>
            )}

            <div className={s.actions}>
              <Button variant="ghost" onClick={reset} aria-label={t('dictation.clearAria')}>
                <Icon icon={Trash2} size={16} />
                {t('dictation.clear')}
              </Button>
              <Button variant="ghost" onClick={copy} aria-label={t('dictation.copyAria')}>
                <Icon icon={Copy} size={16} />
                {copyOk ? t('dictation.copied') : t('dictation.copy')}
              </Button>
              {onInsertIntoComposer && (
                <Button variant="primary" onClick={() => onInsertIntoComposer(transcript)} disabled={!transcript}>
                  <Icon icon={ClipboardPaste} size={16} />
                  {t('dictation.insert')}
                </Button>
              )}
            </div>
          </>
        )}
      </Card>
    </PageShell>
  );
}
