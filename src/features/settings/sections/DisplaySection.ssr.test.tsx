/**
 * DisplaySection SSR evidence — render <DisplaySection /> through the actual
 * react-dom/server renderer and dump the resulting HTML to
 * docs/screenshots/display-section.real.html. This test exists to PROVE the
 * four Cards (Theme / Accent color / Transparency / Background image) come
 * from the real React tree, not a Pillow illustration.
 *
 * Run: pnpm test -- src/features/settings/sections/DisplaySection.ssr.test.tsx
 */
import { describe, it, expect } from 'vitest';
import { renderToString } from 'react-dom/server';
// @ts-ignore — runtime-only node imports; vitest env includes them at test time.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
// @ts-ignore
import { dirname, resolve } from 'node:path';
// @ts-ignore
import { fileURLToPath } from 'node:url';
import React from 'react';
import { I18nProvider } from '@/utils/i18n';
import { DisplaySection } from '@/features/settings/sections/DisplaySection';
import { applyUiPrefs, getUiPrefs } from '@/utils/uiPrefs';

// @ts-ignore
const HERE = dirname(fileURLToPath(import.meta.url));
// @ts-ignore
const ROOT = resolve(HERE, '../../../..');

async function bundleCss(): Promise<string> {
  const [tokens, base, settings] = await Promise.all([
    readFile(resolve(ROOT, 'src/styles/tokens.css'), 'utf8').catch(() => ''),
    readFile(resolve(ROOT, 'src/styles/base.css'), 'utf8').catch(() => ''),
    readFile(resolve(ROOT, 'src/features/settings/SettingsView.module.css'), 'utf8').catch(() => ''),
  ]);
  return [tokens, base, settings].join('\n');
}

describe('DisplaySection SSR evidence', () => {
  it('renders all four primary Cards (Theme / Accent color / Transparency / Background image)', async () => {
    // Pre-seed an interesting default so the SSR snapshot shows a non-virgin state.
    applyUiPrefs({ ...getUiPrefs(), accentColor: '#60a5fa', surfaceOpacity: 0.7, reduceTransparency: false });

    const css = await bundleCss();
    const markup = renderToString(
      React.createElement(I18nProvider, null, React.createElement(DisplaySection)),
    );

    expect(markup).toContain('Theme');
    expect(markup).toContain('Accent color');
    expect(markup).toContain('Transparency');
    expect(markup).toContain('Background image');

    // Control affordance assertions — proves these are real interactive elements
    expect(markup).toMatch(/System[\s\S]*Dark[\s\S]*Light/);
    expect(markup).toMatch(/type="color"/);
    expect(markup).toMatch(/type="range"/);
    expect(markup).toMatch(/type="url"/);
    expect(markup).toMatch(/type="file"/);
    expect(markup).toContain('Surface opacity');
    expect(markup).toContain('Reduce transparency');
    expect(markup).toContain('Choose image');
    expect(markup).toContain('Reset');
    expect(markup).toContain('#60a5fa');
    expect(markup).toContain('aria-label="Custom accent color"');
    expect(markup).toContain('aria-label="Background image URL"');
    // 6 accent preset swatches
    const swatchCount = (markup.match(/class="_colorSwatch_/g) ?? []).length;
    expect(swatchCount).toBe(6);

    // Write evidence files.
    await mkdir(resolve(ROOT, 'docs/screenshots'), { recursive: true });
    const cssInlined = css.replace(/<\/style>/g, '<\\/style>');
    const fullHtml = `<!doctype html>
<html lang="en" data-theme="dark">
<head>
<meta charset="utf-8">
<title>DisplaySection (real SSR)</title>
<style>${cssInlined}</style>
</head>
<body>
<div id="root">${markup}</div>
</body>
</html>`;
    const htmlPath = resolve(ROOT, 'docs/screenshots/display-section.real.html');
    await writeFile(htmlPath, fullHtml, 'utf8');

    // Sanity dump.
    const headings = Array.from(new Set([
      'Theme',
      'Accent color',
      'Transparency',
      'Background image',
    ]));
    const jsonPath = resolve(ROOT, 'docs/screenshots/display-section.real.json');
    await writeFile(
      jsonPath,
      JSON.stringify({ headings, markupBytes: markup.length, htmlBytes: fullHtml.length }, null, 2),
      'utf8',
    );

    console.log('SSR HTML written to:', htmlPath, '(', fullHtml.length, 'bytes )');
    console.log('SSR JSON written to:', jsonPath);
  });
});
