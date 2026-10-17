<script lang="ts">
	// The extension's settings (docs/decisions/0055): signing in to Ottie,
	// and which downloads go there.
	import Download from '@lucide/svelte/icons/download';
	import Link from '@lucide/svelte/icons/link';
	import { onMount } from 'svelte';
	import NumberField from '#ui/components/NumberField.svelte';
	import Switch from '#ui/components/Switch.svelte';
	import { m } from '#ui/paraglide/messages.js';
	import { getLocale } from '#ui/paraglide/runtime.js';
	import { toField } from '#ui/settings.js';
	import { daemonSettings, Unauthorized, Unreachable } from '../lib/daemon';
	import { saveLook } from '../lib/look';
	import {
		DEFAULTS,
		followSettings,
		loadSettings,
		saveSettings,
		type ExtensionSettings,
		type Pairing
	} from '../lib/settings';
	import { parseSignInLink } from '../lib/signin';
	import { parseSites } from '../lib/sites';
	import { megabytes, parseMegabytes, toMegabytes } from '../lib/sizes';

	type Status = 'checking' | 'ok' | 'unreachable' | 'refused';

	let settings: ExtensionSettings = $state(DEFAULTS);
	let status: Status | null = $state(null);
	let link = $state('');
	let linkProblem: string | null = $state(null);
	let busy = $state(false);
	/** Firefox lets people withhold a manifest's site access. */
	let everySite = $state(true);

	const ALL_SITES = { origins: ['<all_urls>'] };

	onMount(() => {
		void loadSettings().then((loaded) => {
			settings = loaded;
			if (loaded.pairing !== null) {
				void check(loaded.pairing);
			}
		});
		void chrome.permissions.contains(ALL_SITES).then((granted) => (everySite = granted));
		return followSettings((changed) => (settings = changed));
	});

	/** Whether Ottie answers with this token; keeps its shared look. */
	async function check(pairing: Pairing): Promise<Status> {
		status = 'checking';
		try {
			await saveLook(await daemonSettings(pairing));
			status = 'ok';
		} catch (error) {
			status = error instanceof Unauthorized ? 'refused' : 'unreachable';
		}
		return status;
	}

	async function signIn(event: SubmitEvent) {
		event.preventDefault();
		const pairing = parseSignInLink(link);
		if (pairing === null) {
			linkProblem = m.others_bad_link();
			return;
		}
		busy = true;
		linkProblem = null;
		const found = await check(pairing);
		busy = false;
		if (found === 'ok') {
			link = '';
			await saveSettings({ pairing });
		} else {
			linkProblem =
				found === 'refused' ? m.ext_refused() : m.ext_unreachable({ address: pairing.address });
			status = null;
		}
	}

	async function signOut() {
		status = null;
		await saveSettings({ pairing: null });
	}

	function openOttie(pairing: Pairing) {
		// Signed in as this browser, as a device's sign-in link does.
		void chrome.tabs.create({ url: `${pairing.address}/#token=${pairing.token}` });
	}

	async function allowEverySite() {
		everySite = await chrome.permissions.request(ALL_SITES);
	}

	async function saveSites(event: Event & { currentTarget: HTMLTextAreaElement }) {
		const excluded = parseSites(event.currentTarget.value);
		event.currentTarget.value = excluded.join('\n');
		await saveSettings({ excluded });
	}

	function statusText(pairing: Pairing, status: Status | null): string {
		switch (status) {
			case 'ok':
				return m.ext_signed_in({ address: pairing.address });
			case 'unreachable':
				return m.ext_unreachable({ address: pairing.address });
			case 'refused':
				return m.ext_refused();
			default:
				return m.others_checking();
		}
	}
</script>

<main>
	<h1>
		<img src="icons/48.png" alt="" width="32" height="32" />
		{m.ext_title()}
	</h1>

	{#if !everySite}
		<p class="notice">
			<span>{m.ext_permission()}</span>
			<button class="primary" onclick={() => void allowEverySite()}>{m.ext_allow()}</button>
		</p>
	{/if}

	<section class="card" aria-labelledby="connection">
		<header>
			<span class="icon" aria-hidden="true"><Link size={16} /></span>
			<h2 id="connection">{m.set_connection()}</h2>
		</header>
		{#if settings.pairing}
			{@const pairing = settings.pairing}
			<div class="setting">
				<span class="names">
					<span dir="auto" class:bad={status === 'unreachable' || status === 'refused'}>
						{statusText(pairing, status)}
					</span>
				</span>
				<span class="buttons">
					{#if status === 'ok'}
						<button onclick={() => openOttie(pairing)}>{m.tray_open()}</button>
					{/if}
					<button onclick={() => void signOut()}>{m.set_device_remove_yes()}</button>
				</span>
			</div>
		{:else}
			<p class="setting hint">{m.ext_signed_out()}</p>
		{/if}
		<form class="setting" onsubmit={(event) => void signIn(event)}>
			<label for="link" class="names">
				<span>{m.set_link_label()}</span>
				<span class="hint">{m.ext_link_hint()}</span>
			</label>
			<span class="link">
				<input
					id="link"
					dir="auto"
					bind:value={link}
					placeholder="http://127.0.0.1:24068/#token=…"
					autocomplete="off"
					spellcheck="false"
				/>
				<button class="primary" disabled={busy || link.trim() === ''}>{m.login_sign_in()}</button>
			</span>
			{#if linkProblem}
				<p class="problem" role="alert">{linkProblem}</p>
			{/if}
		</form>
	</section>

	<section class="card" aria-labelledby="downloads">
		<header>
			<span class="icon" aria-hidden="true"><Download size={16} /></span>
			<h2 id="downloads">{m.set_downloads()}</h2>
		</header>
		<div class="setting">
			<span class="names">
				<span>{m.ext_take_over()}</span>
				<span class="hint">{m.ext_take_over_hint()}</span>
			</span>
			<Switch
				on={settings.takeOver}
				label={m.ext_take_over()}
				onChange={(takeOver) => void saveSettings({ takeOver })}
			/>
		</div>
		<div class="setting">
			<span class="names">
				<span>{m.ext_min_size()}</span>
				<span class="hint">{m.ext_min_size_hint()}</span>
			</span>
			<NumberField
				value={toMegabytes(settings.minSize)}
				label={m.ext_min_size()}
				unit={megabytes(getLocale())}
				parse={(text) => {
					const bytes = parseMegabytes(text);
					return bytes === null ? null : toMegabytes(bytes);
				}}
				show={(value) => toField(value)}
				onChange={async (value) => {
					await saveSettings({ minSize: Math.round(value * 1024 ** 2) });
					return true;
				}}
			/>
		</div>
		<div class="setting">
			<label for="sites" class="names">
				<span>{m.ext_excluded()}</span>
				<span class="hint">{m.ext_excluded_hint()}</span>
			</label>
			<textarea
				id="sites"
				dir="ltr"
				rows="4"
				spellcheck="false"
				placeholder="example.com"
				value={settings.excluded.join('\n')}
				onchange={(event) => void saveSites(event)}></textarea>
		</div>
	</section>
</main>

<style>
	:global(body) {
		margin: 0;
		background: var(--bg);
	}

	main {
		display: grid;
		gap: 0.75rem;
		max-width: 46rem;
		margin: 0 auto;
		padding: 1.5rem 1rem 2rem;
	}

	h1 {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin: 0 0.25rem 0.25rem;
		font-size: 1.5rem;
		font-weight: 600;
	}

	h2 {
		margin: 0;
		font-size: 1.15rem;
		font-weight: 600;
	}

	.card {
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.8rem 1.25rem;
		border-bottom: 1px solid var(--border-soft);
	}

	.icon {
		display: grid;
		flex: none;
		place-items: center;
		width: 2rem;
		height: 2rem;
		background: var(--selected);
		color: var(--accent);
		border-radius: var(--radius-m);
	}

	.setting {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem 1rem;
		margin: 0;
		padding: 0.8rem 1.25rem;
		border-top: 1px solid var(--border-soft);
	}

	header + .setting {
		border-top: none;
	}

	.names {
		display: grid;
		flex: 1 1 16rem;
		font-weight: 500;
	}

	.hint {
		color: var(--faint);
		font-size: 0.85rem;
		font-weight: 400;
	}

	.bad {
		color: var(--danger);
	}

	.buttons,
	.link {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.link {
		flex: 1 1 100%;
	}

	.link input {
		flex: 1 1 14rem;
		min-width: 0;
		padding: 0.4rem 0.75rem;
	}

	textarea {
		flex: 1 1 100%;
		padding: 0.5rem 0.75rem;
		font-family: var(--mono);
		resize: vertical;
	}

	.problem {
		flex-basis: 100%;
		margin: 0;
		color: var(--danger);
	}

	.notice {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		margin: 0;
		padding: 0.75rem 1.25rem;
		border-radius: var(--radius-l);
		background: var(--warn-bg);
		color: var(--warn);
	}
</style>
