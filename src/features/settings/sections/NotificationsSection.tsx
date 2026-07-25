/**
 * NotificationsSection —— Settings > Notifications。
 *
 * 提供声音 / 系统通知 / dock badge 的开关,持久化到 localStorage。
 *
 * 对标 CodexMonitor `SettingsDisplaySection` 中的 notification toggles。
 */
import { useCallback, useEffect, useState } from 'react';
import { Volume2, Bell, Check } from 'lucide-react';
import { Button, Card, Badge, Icon } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import {
  loadNotifyOptions,
  saveNotifyOptions,
  requestNotificationPermission,
  type AgentNotifyOptions,
} from '@/utils/notify';
import s from '../SettingsView.module.css';

export function NotificationsSection() {
  const { t } = useI18n();
  const [opts, setOpts] = useState<AgentNotifyOptions>(() => loadNotifyOptions());
  const [perm, setPerm] = useState<string>(() =>
    typeof window !== 'undefined' && 'Notification' in window
      ? Notification.permission
      : 'unsupported',
  );

  useEffect(() => saveNotifyOptions(opts), [opts]);

  const requestPerm = useCallback(async () => {
    const result = await requestNotificationPermission();
    setPerm(result);
  }, []);

  return (
    <div>
      <Card level="flat" padding="lg">
        <h3 className={s.cardTitle}>
          <Icon icon={Volume2} size={16} /> {t('settings.notifications.sound')}
        </h3>
        <p className={s.cardDesc}>
          {t('settings.notifications.soundHelp')}
        </p>

        <label className={s.row}>
          <input
            type="checkbox"
            checked={opts.sound}
            onChange={(e) => setOpts({ ...opts, sound: e.target.checked })}
          />
          <span>{t('settings.notifications.chime')}</span>
        </label>

        <label className={s.row}>
          <span>{t('settings.notifications.volume')}</span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={opts.volume ?? 0.4}
            onChange={(e) =>
              setOpts({ ...opts, volume: Number.parseFloat(e.target.value) })
            }
          />
          <span className={s.subtle}>
            {Math.round((opts.volume ?? 0.4) * 100)}%
          </span>
        </label>
      </Card>

      <Card level="flat" padding="lg">
        <h3 className={s.cardTitle}>
          <Icon icon={Bell} size={16} /> {t('settings.notifications.desktop')}
        </h3>
        <p className={s.cardDesc}>
          {t('settings.notifications.desktopHelp')}
        </p>

        <div className={s.row}>
          <Badge variant={perm === 'granted' ? 'success' : 'warning'}>
            {perm === 'granted' ? (
              <>
                <Icon icon={Check} size={12} /> {t('settings.notifications.granted')}
              </>
            ) : (
              perm
            )}
          </Badge>
          {perm === 'default' && (
            <Button variant="ghost" onClick={requestPerm}>
              {t('settings.notifications.request')}
            </Button>
          )}
        </div>

        <label className={s.row}>
          <input
            type="checkbox"
            checked={opts.system}
            disabled={perm !== 'granted'}
            onChange={(e) => setOpts({ ...opts, system: e.target.checked })}
          />
          <span>{t('settings.notifications.desktop')}</span>
        </label>
      </Card>
    </div>
  );
}
