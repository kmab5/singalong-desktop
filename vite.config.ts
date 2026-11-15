import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => ({
  plugins: [react()],
  define: { __FIXTURE__: JSON.stringify(mode === 'fixture') },
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
}));
