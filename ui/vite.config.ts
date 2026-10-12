import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';

/** Where `pnpm --filter ui dev` sends API calls: a daemon started with `ottie serve`. */
const DAEMON = process.env.OTTIE_DAEMON ?? 'http://127.0.0.1:16889';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// A single-page app: plain files, built into the folder that `ottie-api`
			// compiles into the binary. Unknown paths get index.html, and the app
			// shows the right page itself.
			adapter: adapter({
				pages: '../crates/api/ui-dist',
				assets: '../crates/api/ui-dist',
				fallback: 'index.html'
			})
		})
	],
	server: {
		// While developing, the page comes from Vite and the API from the daemon.
		proxy: { '/api': { target: DAEMON, ws: true } }
	},
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.ts',
				test: {
					name: 'server',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
				}
			}
		]
	}
});
