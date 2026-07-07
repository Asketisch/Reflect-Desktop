/**
 * TanStack Router v1 — ReflectDesktop route tree。
 *
 * M2.x 里程碑：引入路由基础设施,后续 M3.x 填充各 feature slice。
 *
 * 路由设计：
 * - `/` — Chat (Composer + MessageList, 默认视图)
 * - `/chat/:sessionId` — Chat with specific session
 * - `/sessions` — Session history sidebar expanded
 * - `/settings` — Settings
 * - 其余 feature slice 占位 (M3.x 实装)
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Route, RootRoute, Router } from '@tanstack/react-router';
import React from 'react';

// ====== Route components ======
import { ChatView } from '@/features/messages/ChatView';
import { SettingsView } from '@/features/settings/SettingsView';
import { Sidebar } from '@/features/sessions/components/Sidebar';
import { useSessions } from '@/features/sessions/hooks/useSessions';
import { Topbar, BottomBar } from '@/widgets/StatusBar';

// ====== Placeholder route components for empty feature slices ======
// M3.x 替换为真实实现。

function PlaceholderView({ title, description }: { title: string; description: string }) {
  return (
    <div style={{ padding: 24, maxWidth: 640 }}>
      <h1>{title}</h1>
      <p style={{ color: '#888' }}>{description}</p>
      <p style={{ color: '#aaa', fontSize: 12 }}>M3.x 实装</p>
    </div>
  );
}

const FilesView = () => <PlaceholderView title="Files" description="Browse and edit workspace files." />;
const SkillsView = () => <PlaceholderView title="Skills" description="Manage Reflect skills and plugins." />;
const WorkspacesView = () => <PlaceholderView title="Workspaces" description="Switch between project workspaces." />;
const GitView = () => <PlaceholderView title="Git" description="Git status, diff, and commit workflow." />;
const ModelsView = () => <PlaceholderView title="Models" description="Select and configure LLM models." />;
const TerminalView = () => <PlaceholderView title="Terminal" description="Embedded terminal for shell commands." />;
const PlanView = () => <PlaceholderView title="Plan" description="Plan mode viewer and approval workflow." />;
const PromptsView = () => <PlaceholderView title="Prompts" description="Prompt library and templates." />;
const AboutView = () => <PlaceholderView title="About Reflect" description="Reflect Desktop — AI coding agent GUI for Reflect Agent." />;
const UpdateView = () => <PlaceholderView title="Updates" description="Check for Reflect Desktop updates." />;
const NotificationsView = () => <PlaceholderView title="Notifications" description="Notification center." />;
const DebugView = () => <PlaceholderView title="Debug" description="Debug panel — dev only." />;
const AppsView = () => <PlaceholderView title="Apps" description="App integrations." />;
const CollaborationView = () => <PlaceholderView title="Collaboration" description="Multi-user collaboration." />;
const MobileView = () => <PlaceholderView title="Mobile" description="Mobile companion view." />;
const DictationView = () => <PlaceholderView title="Dictation" description="Voice input." />;
const DesignSystemView = () => <PlaceholderView title="Design System" description="Component library reference." />;

// ====== Layout component ======

// TanStack Router v1 route components receive children as a React prop for parent routes.
// We type as `any` to satisfy v1's `RouteComponent` type which is `(props: {}) => any`.
function AppLayout(props: { children?: React.ReactNode }) {
  const sessions = useSessions();
  const { children } = props;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <Topbar />
      <div style={{ display: 'flex', flex: 1, minHeight: 0 }}>
        <Sidebar
          buckets={sessions.buckets}
          loading={sessions.loading}
          error={sessions.error}
          activeId={sessions.activeId}
          onSelect={sessions.setActiveId}
          onRefresh={sessions.refresh}
        />
        <main style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
          {children}
        </main>
      </div>
      <BottomBar />
    </div>
  );
}

// Settings is a full-screen view without sidebar.
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

const indexRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/',
  component: ChatView,
});

const chatRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'chat',
  component: ChatView,
});

const chatSessionRoute = new Route({
  getParentRoute: () => chatRoute,
  path: '$sessionId',
  component: ChatView,
});

const sessionsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'sessions',
  component: ChatView,
});

const settingsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'settings',
  component: () => <SettingsShell><SettingsView onClose={() => router.navigate({ to: '/' })} /></SettingsShell>,
});

const filesRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'files',
  component: FilesView,
});

const skillsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'skills',
  component: SkillsView,
});

const workspacesRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'workspaces',
  component: WorkspacesView,
});

const gitRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'git',
  component: GitView,
});

const modelsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'models',
  component: ModelsView,
});

const terminalRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'terminal',
  component: TerminalView,
});

const planRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'plan',
  component: PlanView,
});

const promptsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'prompts',
  component: PromptsView,
});

const aboutRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'about',
  component: AboutView,
});

const updateRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'update',
  component: UpdateView,
});

const notificationsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'notifications',
  component: NotificationsView,
});

const debugRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'debug',
  component: DebugView,
});

const appsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'apps',
  component: AppsView,
});

const collaborationRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'collaboration',
  component: CollaborationView,
});

const mobileRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'mobile',
  component: MobileView,
});

const dictationRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'dictation',
  component: DictationView,
});

const designSystemRoute = new Route({
  getParentRoute: () => rootRoute,
  path: 'design-system',
  component: DesignSystemView,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  chatRoute.addChildren([chatSessionRoute]),
  sessionsRoute,
  settingsRoute,
  filesRoute,
  skillsRoute,
  workspacesRoute,
  gitRoute,
  modelsRoute,
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

export const router = new Router({
  routeTree,
});

// ====== Type inference ======

export type AppRouter = typeof router;

// ====== React Query + Router provider ======

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 30, // 30s
      retry: 1,
    },
  },
});

export function AppProviders({ children }: { children: React.ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}
