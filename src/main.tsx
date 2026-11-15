import React from 'react';
import { createRoot } from 'react-dom/client';
import { isTauri } from '@tauri-apps/api/core';
import { App } from './app/App';
import { FloatingReader } from './overlay/FloatingReader';
import { createNative } from './bridge/native';
import './shared/styles.css';

async function start() {
  const root = createRoot(document.getElementById('root')!);
  if (isTauri()) {
    const transport=createNative();
    root.render(<React.StrictMode>{new URLSearchParams(location.search).has('overlay') ? <FloatingReader transport={transport} /> : <App transport={transport} />}</React.StrictMode>);
  } else if (__FIXTURE__) {
    const { createFixture } = await import('./bridge/fixture');
    const fixture = createFixture();
    root.render(<React.StrictMode><App transport={fixture} scenario={fixture.setScenario} /></React.StrictMode>);
  } else {
    root.render(<main className="standalone"><h1>Open Singalong on your desktop</h1><p>This build needs the desktop application. The browser fixture is a separate local development mode.</p></main>);
  }
}
void start();
