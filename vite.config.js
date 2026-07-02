import { defineConfig } from 'vite';
import path from 'path';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  base: './',
  plugins: [wasm()]
});
