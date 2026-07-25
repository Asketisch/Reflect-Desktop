/**
 * UpdatesSection —— Settings > Updates。
 *
 * 通过后端 `reflect_check_update` 命令探测 GitHub release manifest 并
 * 显示是否有可用更新。提供手动 + 自动探测。
 *
 * 不集成 tauri-plugin-updater,因为发布服务尚未搭建;此节提供手动
 * 安装说明 + 检测的 release URL 给用户直接打开。
 */
import { useCallback, useEffect, useState } from 'react';
import { RefreshCw, Download, ExternalLink, AlertCircle } from 'lucide-react';
import { Button, Card, Badge, Icon, Spinner } from '@/features/design-system';
import { useI18n } from '@/utils/i18n';
import { reflect_check_update, type ReflectUpdateInfo } from '@/utils/commands';
import s from '../SettingsView.module.css';

interface State {
  loading: boolean;
  info: ReflectUpdateInfo | null;
  error: string | null;
  lastCheck: number | null;
}

export function UpdatesSection() {
  const { t } = useI18n();
  const [state, setState] = useState<State>({ loading: false, info: null, error: null, lastCheck: null });

  const refresh = useCallback(async () => {
    setState((s) => ({ ...s, loading: true }));
    try {
      const info = await reflect_check_update();
      setState({ loading: false, info, error: info?.error ?? null, lastCheck: Date.now() });
    } catch (e) {
      setState({
        loading: false,
        info: null,
        error: e instanceof Error ? e.message : String(e),
        lastCheck: Date.now(),
      });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const { info, error } = state;
  return (
    <div>
      <Card level="flat" padding="lg">
        <h3 className={s.cardTitle}>
          <Icon icon={RefreshCw} size={16} /> {t('settings.updates.updateCheck')}
        </h3>
        <p className={s.cardDesc}>
          {t('settings.updates.updateCheckHelp')}
        </p>

        {state.loading && (
          <div className={s.row}>
            <Spinner size={14} />
            <span>{t('settings.updates.probing')}</span>
          </div>
        )}

        {info && !state.loading && (
          <div className={s.row}>
            <Badge variant={info.update_available ? 'warning' : 'success'}>
              {info.update_available ? t('settings.updates.availableShort') : t('settings.updates.upToDate')}
            </Badge>
            <span>
              {t('settings.updates.current')} <strong>v{info.current_version}</strong>
              {info.latest_version ? (
                <>
                  {' / '}{t('settings.updates.latest')} <strong>v{info.latest_version}</strong>
                </>
              ) : null}
            </span>
          </div>
        )}

        {error && (
          <div className={s.row}>
            <Badge variant="warning">
              <Icon icon={AlertCircle} size={12} /> {error}
            </Badge>
          </div>
        )}

        <div className={s.row}>
          <Button variant="primary" onClick={refresh}>
            <Icon icon={RefreshCw} size={14} /> {t('settings.updates.checkNow')}
          </Button>
          {info?.release_url && (
            <Button variant="ghost" onClick={() => window.open(info.release_url!, '_blank')}>
              <Icon icon={ExternalLink} size={14} /> {t('settings.updates.releaseNotes')}
            </Button>
          )}
        </div>

        {info?.release_notes && (
          <details className={s.notes}>
            <summary>{t('settings.updates.releaseNotes')}</summary>
            <pre className={s.pre}>{info.release_notes}</pre>
          </details>
        )}
      </Card>

      <Card level="flat" padding="lg">
        <h3 className={s.cardTitle}>
          <Icon icon={Download} size={16} /> {t('settings.updates.manualInstall')}
        </h3>
        <p className={s.cardDesc}>
          {t('settings.updates.manualInstallHelp')}
        </p>
      </Card>
    </div>
  );
}
