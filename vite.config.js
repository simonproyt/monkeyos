import { defineConfig } from 'vite';
import path from 'path';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [wasm()],
  resolve: {
    alias: {
      'wasi_snapshot_preview1': path.resolve(__dirname, 'src/wasi.js')
    }
  }
});
