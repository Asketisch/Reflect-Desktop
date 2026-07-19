/**
 * Update —— 阶段 5b:移除 phantom invoke(reflect_check_update 不存在)。
 *
 * 真实自动更新需要 tauri-plugin-updater(首期未集成)。当前显示版本(来自 ping)
 * + 说明手动更新方式。
 */
import { useQuery } from '@tanstack/react-query';
import { ping } from '@/utils/tauri';

export function UpdateView() {
  const pingQ = useQuery({ queryKey: ['ping'], queryFn: ping, staleTime: Infinity });
  const version = pingQ.data?.version ?? '—';

  return (
    <div style={{ padding: 24, maxWidth: 640, margin: '0 auto' }}>
      <h1 style={{ fontSize: 22, marginBottom: 16 }}>Updates</h1>

      <div
        style={{
          padding: 16,
          border: '1px solid #e2e8f0',
          borderRadius: 8,
          marginBottom: 16,
        }}
      >
        <p style={{ margin: '4px 0', fontSize: 14 }}>
          <strong>Current version:</strong> {version}
        </p>
      </div>

      <section
        style={{
          padding: 12,
          background: '#f8fafc',
          borderRadius: 6,
          fontSize: 13,
          color: '#475569',
        }}
      >
        <h2 style={{ fontSize: 14, margin: '0 0 8px', color: '#666' }}>如何更新</h2>
        <p style={{ margin: '0 0 8px' }}>
          自动更新(tauri-plugin-updater)尚未集成。当前请手动更新:
        </p>
        <pre
          style={{
            background: '#1e1e1e',
            color: '#d4d4d4',
            padding: 10,
            borderRadius: 6,
            fontSize: 12,
            margin: 0,
          }}
        >
{`# 从源码构建最新版
git pull
pnpm install
pnpm tauri build

# 或重装到 /usr/local/bin
bash scripts/install.sh`}
        </pre>
      </section>
    </div>
  );
}
