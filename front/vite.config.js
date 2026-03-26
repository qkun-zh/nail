import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

import topLevelAwait from 'vite-plugin-top-level-await';
import wasm from 'vite-plugin-wasm';
export default defineConfig({
	plugins: [sveltekit(),
		wasm(),
		topLevelAwait()],
	resolve: {
		dedupe: [
			'@codemirror/state',
			'@codemirror/view',
			'@codemirror/language',
			'@lezer/highlight',
			'@lezer/common'
		]
	}
});
