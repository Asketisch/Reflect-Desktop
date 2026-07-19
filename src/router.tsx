/**
 * TanStack Router v1 — ReflectDesktop route tree。
 *
 * M3.x 里程碑：19 条路由 + AppLayout + SettingsShell。
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Route, RootRoute, Router } from '@tanstack/react-router';
import React, { useEffect } from 'react';
import { useAgentStore } from '@/stores/agentStore';
import { ModalStack } from '@/features/modals';

// ====== Route components ======
import { ChatView } from '@/features/messages/ChatView';
import { SettingsView } from '@/features/settings/SettingsView';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions, useActiveSession } from '@/features/sessions/hooks/useSessions';
import { Topbar, BottomBar } from '@/widgets/StatusBar';
import { HomeView } from '@/features/home/HomeView';
import { ThreadsView } from '@/features/threads/ThreadsView';
import { ModelsView } from '@/features/models/ModelsView';
import { FilesView } from '@/features/files/FilesView';
import { GitView } from '@/features/git/GitView';
import { SkillsView } from '@/features/skills/SkillsView';
import { WorkspacesView } from '@/features/workspaces/WorkspacesView';
import { PlanView } from '@/features/plan/PlanView';
import { PromptsView } from '@/features/prompts/PromptsView';
import { NotificationsView } from '@/features/notifications/NotificationsView';
import { TerminalView } from '@/features/terminal/TerminalView';
import { AboutView } from '@/features/about/AboutView';
import { AppsView } from '@/features/apps/AppsView';
import { CollaborationView } from '@/features/collaboration/CollaborationView';
import { DebugView } from '@/features/debug/DebugView';
import { DesignSystemView } from '@/features/design-system/DesignSystemView';
import { DictationView } from '@/features/dictation/DictationView';
import { MobileView } from '@/features/mobile/MobileView';
import { UpdateView } from '@/features/update/UpdateView';

// ====== Placeholder route components ======



// ====== Layout components ======

function AppLayout(props: { children?: React.ReactNode }) {
  const sessions = useSessions();
  const { activeId, setActiveId } = useActiveSession();
  const { children } = props;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <Topbar />
      <div style={{ display: 'flex', flex: 1, minHeight: 0 }}>
        <Sidebar
          buckets={sessions.buckets}
          loading={sessions.loading}
          error={sessions.error}
          activeId={activeId}
          onSelect={setActiveId}
          onRefresh={sessions.refresh}
        />
        <main style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
          {children}
        </main>
      </div>
      <BottomBar />
      {/* 阶段 3c:全局 modal 栈 —— 从 store 的 pending 队列渲染审批/问题/计划 modal。 */}
      <ModalStack />
    </div>
  );
}

function SettingsShell({ children }: { children?: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <Topbar />
      <div style={{ flex: 1, overflow: 'auto' }}>
        {children}
      </div>
      <BottomBar />
    </div>
  );
}

// ====== Route tree (TanStack Router v1 API) ======

const rootRoute = new RootRoute({
  component: AppLayout,
});

const indexRoute = new Route({ getParentRoute: () => rootRoute, path: '/', component: HomeView });
const chatRoute = new Route({ getParentRoute: () => rootRoute, path: 'chat', component: ChatView });
const chatSessionRoute = new Route({ getParentRoute: () => chatRoute, path: '$sessionId', component: ChatView });
const sessionsRoute = new Route({ getParentRoute: () => rootRoute, path: 'sessions', component: ThreadsView });
const settingsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'settings',
  component: () => <SettingsShell><SettingsView onClose={() => router.navigate({ to: '/' })} /></SettingsShell>,
});
const filesRoute = new Route({ getParentRoute: () => rootRoute, path: 'files', component: FilesView });
const modelsRoute = new Route({ getParentRoute: () => rootRoute, path: 'models', component: ModelsView });
const skillsRoute = new Route({ getParentRoute: () => rootRoute, path: 'skills', component: SkillsView });
const workspacesRoute = new Route({ getParentRoute: () => rootRoute, path: 'workspaces', component: WorkspacesView });
const gitRoute = new Route({ getParentRoute: () => rootRoute, path: 'git', component: GitView });
const terminalRoute = new Route({ getParentRoute: () => rootRoute, path: 'terminal', component: TerminalView });
const planRoute = new Route({ getParentRoute: () => rootRoute, path: 'plan', component: PlanView });
const promptsRoute = new Route({ getParentRoute: () => rootRoute, path: 'prompts', component: PromptsView });
const aboutRoute = new Route({ getParentRoute: () => rootRoute, path: 'about', component: AboutView });
const updateRoute = new Route({ getParentRoute: () => rootRoute, path: 'update', component: UpdateView });
const notificationsRoute = new Route({ getParentRoute: () => rootRoute, path: 'notifications', component: NotificationsView });
const debugRoute = new Route({ getParentRoute: () => rootRoute, path: 'debug', component: DebugView });
const appsRoute = new Route({ getParentRoute: () => rootRoute, path: 'apps', component: AppsView });
const collaborationRoute = new Route({ getParentRoute: () => rootRoute, path: 'collaboration', component: CollaborationView });
const mobileRoute = new Route({ getParentRoute: () => rootRoute, path: 'mobile', component: MobileView });
const dictationRoute = new Route({ getParentRoute: () => rootRoute, path: 'dictation', component: DictationView });
const designSystemRoute = new Route({ getParentRoute: () => rootRoute, path: 'design-system', component: DesignSystemView });

const routeTree = rootRoute.addChildren([
  indexRoute,
  chatRoute.addChildren([chatSessionRoute]),
  sessionsRoute,
  settingsRoute,
  filesRoute,
  modelsRoute,
  skillsRoute,
  workspacesRoute,
  gitRoute,
  terminalRoute,
  planRoute,
  promptsRoute,
  aboutRoute,
  updateRoute,
  notificationsRoute,
  debugRoute,
  appsRoute,
  collaborationRoute,
  mobileRoute,
  dictationRoute,
  designSystemRoute,
]);

// ====== Router instance ======

export const router = new Router({ routeTree });

// ====== Type inference ======

export type AppRouter = typeof router;

// ====== React Query provider ======

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 30,
      retry: 1,
    },
  },
});

export function AppProviders({ children }: { children: React.ReactNode }) {
  // 阶段 3a:在 provider mount 时建立唯一的 reflect_event 订阅。
  // store.subscribe() 是幂等的(防止 React 18 StrictMode 双 mount 重复订阅)。
  // unmount 时清理(返回的 cleanup 调 unlisten + 标记未订阅)。
  useEffect(() => {
    const unsubscribe = useAgentStore.getState().subscribe();
    return unsubscribe;
  }, []);
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}
