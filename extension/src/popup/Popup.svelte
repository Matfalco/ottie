<script lang="ts">
	// The extension's popup (docs/decisions/0055): a bar of its own (taking
	// downloads over on or off, its settings), then Ottie's own `/popup`
	// page, signed in with this browser's token, so it always matches
	// Ottie's version and speaks its language.
	import Settings from '@lucide/svelte/icons/settings';
	import { onMount } from 'svelte';
	import Switch from '#ui/components/Switch.svelte';
	import { m } from '#ui/paraglide/messages.js';
	import { daemonSettings, Unauthorized } from '../lib/daemon';
	import { frameHeight, fromPage } from '../lib/frame';
	import { saveLook } from '../lib/look';
	import { DEFAULTS, loadSettings, saveSettings, type ExtensionSettings } from '../lib/settings';

	type Status = 'checking' | 'ok' | 'unreachable' | 'refused';

	let settings: ExtensionSettings = $state(DEFAULTS);
	let loaded = $state(false);
	let status: Status = $state('checking');
	let frame: HTMLIFrameElement | undefined = $state();
	let height = $state(120);

	onMount(() => {
		void loadSettings().then(async (found) => {
			settings = found;
			loaded = true;
			if (found.pairing === null) {
				return;
			}
			try {
				await saveLook(await daemonSettings(found.pairing));
				status = 'ok';
			} catch (error) {
				status = error instanceof Unauthorized ? 'refused' : 'unreachable';
			}
		});
	});

	// Only the page in the frame, from Ottie's address, is listened to, and
	// only it gets the token.
	onMount(() => {
		const listen = (event: MessageEvent) => {
			const pairing = settings.pairing;
			const said = fromPage(event.data);
			if (!pairing || !said || event.origin !== pairing.address) {
				return;
			}
			if (event.source !== frame?.contentWindow) {
				return;
			}
			if (said.type === 'ottie-popup-ready') {
				frame.contentWindow?.postMessage(
					{ type: 'ottie-token', token: pairing.token },
					pairing.address
				);
			} else if (said.type === 'ottie-popup-size') {
				height = frameHeight(said.height);
			} else {
				status = 'refused';
			}
		};
		addEventListener('message', listen);
		return () => removeEventListener('message', listen);
	});

	function openSettings() {
		void chrome.runtime.openOptionsPage();
		window.close();
	}
</script>

<div class="popup">
	<header>
		<img src="icons/32.png" alt="" width="20" height="20" />
		<span class="name">Ottie</span>
		<label class="switch">
			<span class="label">{m.ext_take_over()}</span>
			<Switch
				on={settings.takeOver}
				label={m.ext_take_over()}
				onChange={(takeOver) => {
					settings.takeOver = takeOver;
					void saveSettings({ takeOver });
				}}
			/>
		</label>
		<button class="round" onclick={openSettings} aria-label={m.set_title()} title={m.set_title()}>
			<Settings size={15} aria-hidden="true" />
		</button>
	</header>
	{#if loaded}
		{#if settings.pairing === null}
			<p class="message">
				<span>{m.ext_signed_out()}</span>
				<button class="primary" onclick={openSettings}>{m.login_sign_in()}</button>
			</p>
		{:else if status === 'ok'}
			<iframe
				bind:this={frame}
				src={`${settings.pairing.address}/popup`}
				title="Ottie"
				style:height={`${height}px`}
			></iframe>
		{:else if status !== 'checking'}
			<p class="message">
				<span class="bad" dir="auto">
					{status === 'refused'
						? m.ext_refused()
						: m.ext_unreachable({ address: settings.pairing.address })}
				</span>
				<button onclick={openSettings}>{m.set_title()}</button>
			</p>
		{/if}
	{/if}
</div>

<style>
	:global(body) {
		margin: 0;
		background: var(--surface);
	}

	.popup {
		width: 380px;
		color: var(--text);
		font-size: 0.875rem;
	}

	header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--border-soft);
	}

	.name {
		flex: 1;
		font-weight: 600;
	}

	.switch {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--muted);
		font-size: 0.8rem;
		cursor: pointer;
	}

	.round {
		display: grid;
		flex: none;
		place-items: center;
		width: 1.9rem;
		height: 1.9rem;
		padding: 0;
	}

	iframe {
		display: block;
		width: 100%;
		border: none;
	}

	.message {
		display: grid;
		gap: 0.75rem;
		justify-items: start;
		margin: 0;
		padding: 1rem 0.75rem;
	}

	.bad {
		color: var(--danger);
	}
</style>
