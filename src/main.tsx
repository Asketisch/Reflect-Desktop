import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { RouterProvider } from '@tanstack/react-router';
import { AppProviders, router } from './router';
import { initTheme } from './utils/theme';
import { bootstrapUiPrefs } from './utils/uiPrefs';
import './styles/tokens.css';
import './styles/base.css';

// 启动时把持久化的主题模式 + UI 偏好 apply 到 <html>。
initTheme();
bootstrapUiPrefs();

const root = document.getElementById('root');
if (!root) {
  throw new Error('#root element missing');
}

createRoot(root).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  </StrictMode>
);
