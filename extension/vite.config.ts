import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, type Plugin } from 'vitest/config';
import { manifest, type Browser } from './src/manifest.ts';

const here = (path: string) => fileURLToPath(new URL(path, import.meta.url));

/** The manifest and the icons (the app's, res/icons) beside the build. */
function extensionFiles(browser: Browser): Plugin {
	const { version } = JSON.parse(readFileSync(here('package.json'), 'utf8')) as {
		version: string;
	};
	return {
		name: 'ottie-extension-files',
		generateBundle() {
			this.emitFile({
				type: 'asset',
				fileName: 'manifest.json',
				source: JSON.stringify(manifest(browser, version), null, '\t')
			});
			for (const size of [16, 32, 48, 128]) {
				this.emitFile({
					type: 'asset',
					fileName: `icons/${size}.png`,
					source: readFileSync(here(`../res/icons/png/app/ottie-icon-rounded-${size}.png`))
				});
			}
		}
	};
}

// `vite build --mode chrome` → dist/chrome, `--mode firefox` → dist/firefox.
export default defineConfig(({ mode }) => {
	const browser: Browser = mode === 'firefox' ? 'firefox' : 'chrome';
	return {
		plugins: [svelte({ compilerOptions: { runes: true } }), extensionFiles(browser)],
		// The page's code and texts, shared with the daemon's pages.
		resolve: { alias: { '#ui': here('../ui/src/lib') } },
		build: {
			outDir: `dist/${browser}`,
			emptyOutDir: true,
			target: 'es2022',
			modulePreload: false,
			rollupOptions: {
				input: {
					background: here('src/background.ts'),
					options: here('options.html')
				},
				output: {
					entryFileNames: '[name].js',
					chunkFileNames: 'chunks/[name]-[hash].js',
					assetFileNames: 'assets/[name]-[hash][extname]'
				}
			}
		},
		test: {
			expect: { requireAssertions: true },
			include: ['src/**/*.test.ts'],
			environment: 'node'
		}
	};
});
