import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import {defineConfig} from 'vitest/config';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  // Komponententests brauchen ein DOM; reine Logik-Tests laufen ebenfalls darin
  test: {environment: 'jsdom'},
  server: {port: 5173, strictPort: true},
});
