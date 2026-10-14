<script lang="ts">
	import type { SettingsChange } from '@ottie/api-types/SettingsChange';
	import AppWindow from '@lucide/svelte/icons/app-window';
	import Bell from '@lucide/svelte/icons/bell';
	import Copy from '@lucide/svelte/icons/copy';
	import Download from '@lucide/svelte/icons/download';
	import Paintbrush from '@lucide/svelte/icons/palette';
	import Play from '@lucide/svelte/icons/play';
	import Plug from '@lucide/svelte/icons/plug';
	import RotateCw from '@lucide/svelte/icons/rotate-cw';
	import { appearance, setAppearance } from '#lib/appearance.svelte.js';
	import { useApp } from '#lib/context.js';
	import { m } from '#lib/paraglide/messages.js';
	import { setLocale, type Locale } from '#lib/paraglide/runtime.js';
	import { RANGES, parseSeconds, parseWhole, toField } from '#lib/settings.js';
	import {
		MODES,
		PALETTES,
		THEMES,
		lookOf,
		previewFor,
		type DeviceAppearance,
		type Mode,
		type Palette
	} from '#lib/theme.js';
	import { signInLink } from '#lib/token.js';
	import {
		canChoosePlayer,
		choosePlayer,
		desktop as desktopSetting,
		player as playerSetting,
		setCloseToTray,
		setNotify,
		setNotifyFailed,
		setPlayWith,
		type Desktop,
		type Notify,
		type PlayWith,
		type Player
	} from '#lib/window.js';
	import LanguagePicker from './LanguagePicker.svelte';
	import LimitControl from './LimitControl.svelte';
	import NumberField from './NumberField.svelte';
	import Switch from './Switch.svelte';

	const { session, token } = useApp();
	/** The daemon's settings; null until it has answered. */
	const settings = $derived(session.settings);
	const link = signInLink(location.origin, token);

	const MODE_NAMES: Record<Mode, () => string> = {
		system: m.set_mode_system,
		light: m.set_mode_light,
		dark: m.set_mode_dark
	};

	const PALETTE_NAMES: Record<Palette, { name: () => string; hint: () => string }> = {
		sea: { name: m.set_palette_sea, hint: m.set_palette_sea_hint },
		neutral: { name: m.set_palette_neutral, hint: m.set_palette_neutral_hint }
	};

	/** The choices besides a program of one's own. */
	const PLAY_NAMES: Record<'system' | 'browser', { name: () => string; hint: () => string }> = {
		system: { name: m.set_play_system, hint: m.set_play_system_hint },
		browser: { name: m.set_play_browser, hint: m.set_play_browser_hint }
	};

	/** When finished downloads are notified, in the order shown. */
	const NOTIFY_NAMES: Record<Notify, { name: () => string; hint: () => string }> = {
		all: { name: m.set_notify_all, hint: m.set_notify_all_hint },
		each: { name: m.set_notify_each, hint: m.set_notify_each_hint },
		never: { name: m.set_notify_never, hint: m.set_notify_never_hint }
	};
	const NOTIFY_CHOICES = Object.keys(NOTIFY_NAMES) as Notify[];

	/** Only the desktop app can start a player (docs/decisions/0031), stay
	 * in the tray (0034) or notify (0035). */
	const inApp = canChoosePlayer();

	/** The sections, top to bottom, for the list on the left. */
	const SECTIONS: { id: string; label: () => string }[] = [
		{ id: 'set-appearance', label: m.set_appearance },
		{ id: 'set-downloads', label: m.set_downloads },
		...(inApp
			? [
					{ id: 'set-app', label: m.set_app },
					{ id: 'set-notifications', label: m.set_notifications },
					{ id: 'set-playing', label: m.set_playing }
				]
			: []),
		{ id: 'set-retries', label: m.set_retries },
		{ id: 'set-connection', label: m.set_connection }
	];

	let page: HTMLElement;
	/** The section being read: the last one whose top has scrolled past
	 * the top of the page, or the last one once at the bottom. */
	let current = $state(SECTIONS[0].id);
	/** Why the last change wasn't saved. */
	let problem: string | null = $state(null);
	let copied = $state(false);
	/** The clipboard refused: the link is shown to copy by hand. */
	let showLink = $state(false);
	/** How the desktop app plays; null until it has answered (or in a browser). */
	let playing: Player | null = $state(null);
	/** The choice shown, put back if a change doesn't happen. */
	let playWith: PlayWith = $state('system');

	/** The desktop app's own settings; null until it has answered (or in a browser). */
	let desktop: Desktop | null = $state(null);
	/** The notify choice shown, put back if a change doesn't happen. */
	let notify: Notify = $state('all');

	if (inApp) {
		playerSetting().then(showPlayer, () => (problem = m.set_play_failed()));
		desktopSetting().then(showDesktop, () => (problem = m.set_app_failed()));
	}

	function showDesktop(answer: Desktop) {
		desktop = answer;
		notify = answer.notify;
	}

	async function changeDesktop(change: () => Promise<Desktop>) {
		try {
			showDesktop(await change());
		} catch {
			problem = m.set_app_failed();
			notify = desktop?.notify ?? 'all';
		}
	}

	function showPlayer(player: Player) {
		playing = player;
		playWith = player.playWith;
	}

	/** A program is asked for the first time it's chosen. */
	async function changePlayer(next: PlayWith) {
		try {
			showPlayer(
				next === 'program' && !playing?.program
					? await choosePlayer(m.set_play_pick_title(), m.set_play_programs())
					: await setPlayWith(next)
			);
		} catch {
			problem = m.set_play_failed();
			playWith = playing?.playWith ?? 'system';
		}
	}

	async function pickProgram() {
		try {
			showPlayer(await choosePlayer(m.set_play_pick_title(), m.set_play_programs()));
		} catch {
			problem = m.set_play_failed();
		}
	}

	function scrolled() {
		const top = page.getBoundingClientRect().top;
		const atBottom = page.scrollTop + page.clientHeight >= page.scrollHeight - 2;
		let found = SECTIONS[0].id;
		for (const { id } of SECTIONS) {
			const section = document.getElementById(id);
			if (section && (atBottom || section.getBoundingClientRect().top - top <= 80)) {
				found = id;
			}
		}
		current = found;
	}

	function go(event: MouseEvent, id: string) {
		event.preventDefault();
		document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
		current = id;
	}

	async function save(change: SettingsChange): Promise<boolean> {
		problem = await session.changeSettings(change);
		return problem === null;
	}

	/** Changes the look here, and for every device that follows it. */
	function choose(change: Partial<DeviceAppearance>) {
		setAppearance(change);
		if (appearance.synced) {
			void save({ appearance: lookOf(appearance) });
		}
	}

	/** A language for this page, and for every device that follows. */
	async function chooseLanguage(locale: Locale) {
		if (appearance.synced && !(await save({ language: locale }))) {
			return;
		}
		setLocale(locale); // Reloads the page in that language.
	}

	function step(by: number) {
		if (settings) {
			const { min, max } = RANGES.maxConcurrent;
			const next = Math.min(max, Math.max(min, settings.maxConcurrent + by));
			void save({ maxConcurrent: next });
		}
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(link);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			showLink = true; // Not allowed here (a plain-http remote page).
		}
	}
</script>

<div class="settings" bind:this={page} onscroll={scrolled}>
	<nav aria-label={m.set_sections()}>
		{#each SECTIONS as { id, label } (id)}
			<a
				href="#{id}"
				class:current={current === id}
				aria-current={current === id ? 'true' : undefined}
				onclick={(event) => go(event, id)}
			>
				{label()}
			</a>
		{/each}
	</nav>

	<div class="sections">
		<h1>{m.set_title()}</h1>

		{#if problem}
			<div class="problem" role="alert">
				<span>{problem}</span>
				<button onclick={() => (problem = null)} aria-label={m.dismiss()}>✕</button>
			</div>
		{/if}

		<section id="set-appearance" class="card" aria-labelledby="appearance">
			<header class="with-switch">
				<span class="icon" aria-hidden="true"><Paintbrush size={16} /></span>
				<span class="names">
					<h2 id="appearance">{m.set_appearance()}</h2>
					<span class="hint">{appearance.synced ? m.set_sync_hint() : m.set_appearance_hint()}</span
					>
				</span>
				<!-- Following the shared look and language: the shell takes them
				     on, or shares this device's while none are shared. -->
				<label class="sync">
					<span>{m.set_sync()}</span>
					<Switch
						on={appearance.synced}
						label={m.set_sync()}
						onChange={(synced) => setAppearance({ synced })}
					/>
				</label>
			</header>

			<fieldset>
				<legend>{m.set_mode()}</legend>
				<div class="options modes">
					{#each MODES as mode (mode)}
						{@const [left, right] = previewFor(mode, appearance.palette)}
						<label class="option" class:chosen={appearance.mode === mode}>
							<!-- Each half carries its theme, so it shows that theme's colours. -->
							<span class="preview" aria-hidden="true">
								<span class="half" data-theme={left}>
									<span class="bar"></span><span class="line"></span><span class="line short"
									></span>
								</span>
								<span class="half" data-theme={right}>
									<span class="bar"></span><span class="line"></span><span class="line short"
									></span>
								</span>
							</span>
							<span class="choice">
								<input
									type="radio"
									name="mode"
									checked={appearance.mode === mode}
									onchange={() => choose({ mode })}
								/>
								{MODE_NAMES[mode]()}
							</span>
						</label>
					{/each}
				</div>
			</fieldset>

			<fieldset>
				<legend>{m.set_palette()}</legend>
				<div class="options palettes">
					{#each PALETTES as palette (palette)}
						<label class="option row" class:chosen={appearance.palette === palette}>
							<input
								type="radio"
								name="palette"
								checked={appearance.palette === palette}
								onchange={() => choose({ palette })}
							/>
							<span class="names">
								<span>{PALETTE_NAMES[palette].name()}</span>
								<span class="hint">{PALETTE_NAMES[palette].hint()}</span>
							</span>
							<span class="swatches" aria-hidden="true">
								<span data-theme={THEMES[palette].light}></span>
								<span data-theme={THEMES[palette].dark}></span>
							</span>
						</label>
					{/each}
				</div>
			</fieldset>

			<div class="setting">
				<label for="language" class="names">
					<span>{m.set_language_choice()}</span>
					<span class="hint">{m.set_language_hint()}</span>
				</label>
				<LanguagePicker id="language" onChoose={(locale) => void chooseLanguage(locale)} />
			</div>
		</section>

		<section id="set-downloads" class="card" aria-labelledby="downloads-title">
			<header>
				<span class="icon" aria-hidden="true"><Download size={16} /></span>
				<h2 id="downloads-title">{m.set_downloads()}</h2>
			</header>
			{#if settings}
				<div class="setting">
					<span class="names">
						<span>{m.set_max_concurrent()}</span>
						<span class="hint">{m.set_max_concurrent_hint()}</span>
					</span>
					<span class="stepper" role="group" aria-label={m.set_max_concurrent()}>
						<button
							aria-label={m.set_fewer()}
							disabled={settings.maxConcurrent <= RANGES.maxConcurrent.min}
							onclick={() => step(-1)}>−</button
						>
						<NumberField
							value={settings.maxConcurrent}
							label={m.set_max_concurrent()}
							parse={(text) => parseWhole(text, RANGES.maxConcurrent)}
							onChange={(maxConcurrent) => save({ maxConcurrent })}
						/>
						<button
							aria-label={m.set_more()}
							disabled={settings.maxConcurrent >= RANGES.maxConcurrent.max}
							onclick={() => step(1)}>+</button
						>
					</span>
				</div>
				<div class="setting">
					<span class="names">
						<span>{m.limit_total()}</span>
						<span class="hint">{m.limit_total_hint()}</span>
					</span>
					<LimitControl
						limit={settings.globalLimit}
						onChange={(globalLimit) => save({ globalLimit })}
					/>
				</div>
				<div class="setting">
					<span class="names">
						<span>{m.set_stream_media()}</span>
						<span class="hint">{m.set_stream_media_hint()}</span>
					</span>
					<Switch
						on={settings.streamMedia}
						label={m.set_stream_media()}
						onChange={(streamMedia) => save({ streamMedia })}
					/>
				</div>
			{:else}
				<p class="waiting">{m.connection_connecting()}</p>
			{/if}
		</section>

		{#if inApp}
			<section id="set-app" class="card" aria-labelledby="app-title">
				<header>
					<span class="icon" aria-hidden="true"><AppWindow size={16} /></span>
					<h2 id="app-title">{m.set_app()}</h2>
					<p class="hint">{m.set_app_hint()}</p>
				</header>
				{#if desktop}
					<div class="setting">
						<span class="names">
							<span>{m.set_close_to_tray()}</span>
							<span class="hint">{m.set_close_to_tray_hint()}</span>
						</span>
						<Switch
							on={desktop.closeToTray}
							label={m.set_close_to_tray()}
							onChange={(on) => changeDesktop(() => setCloseToTray(on))}
						/>
					</div>
				{:else}
					<p class="waiting">{m.connection_connecting()}</p>
				{/if}
			</section>

			<section id="set-notifications" class="card" aria-labelledby="notifications-title">
				<header>
					<span class="icon" aria-hidden="true"><Bell size={16} /></span>
					<h2 id="notifications-title">{m.set_notifications()}</h2>
					<p class="hint">{m.set_notifications_hint()}</p>
				</header>
				{#if desktop}
					<fieldset>
						<legend>{m.set_notify()}</legend>
						<div class="options notifies">
							{#each NOTIFY_CHOICES as choice (choice)}
								<label class="option row" class:chosen={notify === choice}>
									<input
										type="radio"
										name="notify"
										value={choice}
										bind:group={notify}
										onchange={() => changeDesktop(() => setNotify(choice))}
									/>
									<span class="names">
										<span>{NOTIFY_NAMES[choice].name()}</span>
										<span class="hint">{NOTIFY_NAMES[choice].hint()}</span>
									</span>
								</label>
							{/each}
						</div>
					</fieldset>
					<div class="setting">
						<span class="names">
							<span>{m.set_notify_failed()}</span>
							<span class="hint">{m.set_notify_failed_hint()}</span>
						</span>
						<Switch
							on={desktop.notifyFailed}
							label={m.set_notify_failed()}
							onChange={(on) => changeDesktop(() => setNotifyFailed(on))}
						/>
					</div>
				{:else}
					<p class="waiting">{m.connection_connecting()}</p>
				{/if}
			</section>

			<section id="set-playing" class="card" aria-labelledby="playing-title">
				<header>
					<span class="icon" aria-hidden="true"><Play size={16} /></span>
					<h2 id="playing-title">{m.set_playing()}</h2>
					<p class="hint">{m.set_playing_hint()}</p>
				</header>
				{#if playing}
					<fieldset class="last">
						<legend>{m.set_play_with()}</legend>
						<div class="options plays">
							{#each ['system', 'browser'] as const as way (way)}
								<label class="option row" class:chosen={playWith === way}>
									<input
										type="radio"
										name="play"
										value={way}
										bind:group={playWith}
										onchange={() => changePlayer(way)}
									/>
									<span class="names">
										<span>{PLAY_NAMES[way].name()}</span>
										<span class="hint">{PLAY_NAMES[way].hint()}</span>
									</span>
								</label>
							{/each}
							<!-- Not one label: the button inside mustn't pick the choice. -->
							<div class="option row" class:chosen={playWith === 'program'}>
								<label class="program">
									<input
										type="radio"
										name="play"
										value="program"
										bind:group={playWith}
										onchange={() => changePlayer('program')}
									/>
									<span class="names">
										<span>{m.set_play_program()}</span>
										<span class="hint path">{playing.program ?? m.set_play_program_none()}</span>
									</span>
								</label>
								<button onclick={pickProgram}>{m.set_play_choose()}</button>
							</div>
						</div>
					</fieldset>
				{:else}
					<p class="waiting">{m.connection_connecting()}</p>
				{/if}
			</section>
		{/if}

		<section id="set-retries" class="card" aria-labelledby="retries-title">
			<header>
				<span class="icon" aria-hidden="true"><RotateCw size={16} /></span>
				<h2 id="retries-title">{m.set_retries()}</h2>
				<p class="hint">{m.set_retries_hint()}</p>
			</header>
			{#if settings}
				<div class="setting">
					<span class="names">
						<span>{m.set_max_retries()}</span>
						<span class="hint">{m.set_max_retries_hint()}</span>
					</span>
					<NumberField
						value={settings.maxRetries}
						label={m.set_max_retries()}
						unit=""
						parse={(text) => parseWhole(text, RANGES.maxRetries)}
						onChange={(maxRetries) => save({ maxRetries })}
					/>
				</div>
				<div class="setting">
					<span class="names">
						<span>{m.set_first_wait()}</span>
						<span class="hint">{m.set_first_wait_hint()}</span>
					</span>
					<NumberField
						value={settings.firstRetryWaitMs}
						label={m.set_first_wait()}
						unit={m.unit_seconds()}
						parse={(text) => parseSeconds(text, RANGES.firstRetryWaitMs)}
						show={(ms) => toField(ms / 1000)}
						onChange={(firstRetryWaitMs) => save({ firstRetryWaitMs })}
					/>
				</div>
				<div class="setting">
					<span class="names">
						<span>{m.set_yield()}</span>
						<span class="hint">{m.set_yield_hint()}</span>
					</span>
					<NumberField
						value={settings.retriesBeforeYield}
						label={m.set_yield()}
						unit=""
						parse={(text) => parseWhole(text, RANGES.retriesBeforeYield)}
						onChange={(retriesBeforeYield) => save({ retriesBeforeYield })}
					/>
				</div>
			{:else}
				<p class="waiting">{m.connection_connecting()}</p>
			{/if}
		</section>

		<section id="set-connection" class="card" aria-labelledby="connection-title">
			<header>
				<span class="icon" aria-hidden="true"><Plug size={16} /></span>
				<h2 id="connection-title">{m.set_connection()}</h2>
			</header>
			<div class="setting">
				<span class="names">
					<span>{m.set_daemon()}</span>
					<span class="status {session.connection}">
						<span class="dot" aria-hidden="true"></span>
						{#if session.connection === 'live'}
							{m.set_connected_to({ address: location.host })}
						{:else if session.connection === 'connecting'}
							{m.connection_connecting()}
						{:else}
							{m.connection_offline()}
						{/if}
					</span>
				</span>
				<button class="copy" onclick={copy}>
					<Copy size={15} aria-hidden="true" />
					{copied ? m.set_link_copied() : m.set_copy_link()}
				</button>
			</div>
			{#if showLink}
				<div class="link">
					<input
						readonly
						value={link}
						aria-label={m.set_link_label()}
						onfocus={(event) => event.currentTarget.select()}
					/>
				</div>
			{/if}
			<p class="hint below">{m.set_link_hint()}</p>
		</section>
	</div>
</div>

<style>
	/* The section list on the left, the sections taking the rest. */
	.settings {
		display: flex;
		flex: 1;
		align-items: flex-start;
		gap: 1.25rem;
		min-height: 0;
		padding: 0.25rem 1rem 1.5rem;
		overflow: auto;
	}

	nav {
		position: sticky;
		top: 0;
		display: grid;
		flex: 0 0 9.5rem;
		gap: 0.15rem;
		padding-top: 3.1rem;
	}

	nav a {
		padding: 0.45rem 0.75rem;
		border-radius: var(--radius-s);
		color: var(--muted);
		text-decoration: none;
	}

	nav a:hover {
		background: var(--hover);
		color: var(--text);
	}

	nav a.current {
		background: var(--surface);
		color: var(--text);
		font-weight: 600;
	}

	.sections {
		display: grid;
		flex: 1;
		gap: 0.75rem;
		min-width: 0;
	}

	h1 {
		margin: 0.25rem 0.25rem 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.problem {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.6rem 0.9rem;
		border-radius: var(--radius-s);
		background: var(--danger-bg);
		color: var(--danger);
	}

	.problem button {
		padding: 0 0.4rem;
		border: none;
		background: none;
		color: inherit;
	}

	.card {
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	/* A band across the top of each card, so its title stands apart from
	   the settings under it. */
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
		padding: 0.8rem 1.25rem;
		border-bottom: 1px solid var(--border);
		border-radius: calc(var(--radius-l) - 1px) calc(var(--radius-l) - 1px) 0 0;
		background: var(--well);
	}

	.icon {
		display: grid;
		flex: none;
		place-items: center;
		width: 2rem;
		height: 2rem;
		border-radius: var(--radius-s);
		background: var(--selected);
		color: var(--accent);
	}

	/* The card's title and hint, and a switch for all of it on the right. */
	header.with-switch {
		column-gap: 1rem;
	}

	header.with-switch .icon {
		margin-right: -0.25rem;
	}

	.sync {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		font-weight: 500;
		cursor: pointer;
	}

	h2 {
		margin: 0;
		font-size: 1.15rem;
		font-weight: 600;
	}

	.hint {
		margin: 0;
		color: var(--faint);
		font-size: 0.85rem;
	}

	fieldset {
		margin: 0;
		padding: 0.75rem 1.25rem 1rem;
		border: none;
		border-bottom: 1px solid var(--border-soft);
	}

	/* Headings within a card: smaller than its title, unlike a setting's name. */
	legend {
		float: left;
		margin-bottom: 0.6rem;
		padding: 0;
		color: var(--muted);
		font-size: 0.85rem;
		font-weight: 600;
	}

	.options {
		display: grid;
		clear: both;
		gap: 0.65rem;
	}

	.modes {
		grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
	}

	.palettes {
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
	}

	.option {
		display: grid;
		gap: 0.5rem;
		padding: 0.5rem;
		border: 2px solid var(--border-soft);
		border-radius: var(--radius-m);
		cursor: pointer;
	}

	.option:hover {
		border-color: var(--border);
	}

	.option.chosen {
		border-color: var(--accent);
	}

	.option.row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 0.75rem;
	}

	.option input,
	.choice input {
		margin: 0;
		accent-color: var(--accent);
	}

	.preview {
		display: flex;
		height: 4rem;
		overflow: hidden;
		border: 1px solid var(--border-soft);
		border-radius: var(--radius-s);
	}

	.half {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 0.3rem;
		padding: 0.5rem;
		background: var(--bg);
	}

	.bar,
	.line {
		height: 0.35rem;
		border-radius: 3px;
		background: var(--border);
	}

	.bar {
		width: 60%;
		height: 0.4rem;
		background: var(--accent);
	}

	.line.short {
		width: 80%;
	}

	.choice {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0 0.15rem;
		font-weight: 500;
	}

	.names {
		display: grid;
		flex: 1 1 16rem;
		font-weight: 500;
	}

	.names .hint {
		font-weight: 400;
	}

	/* Each palette's light and dark theme: its background with its accent. */
	.swatches {
		display: flex;
		gap: 0.25rem;
	}

	.swatches span {
		width: 2.2rem;
		height: 1.8rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: linear-gradient(to bottom, var(--bg) 60%, var(--accent) 60%);
	}

	/* One setting per row: its name and hint, then the control. */
	.setting {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem 1rem;
		padding: 0.8rem 1.25rem;
		border-top: 1px solid var(--border-soft);
	}

	header + .setting,
	fieldset + .setting {
		border-top: none;
	}

	.waiting {
		margin: 0;
		padding: 0.5rem 1.25rem 1.1rem;
		color: var(--faint);
	}

	/* Fewer, the number, more: one control. */
	.stepper {
		display: flex;
		align-items: center;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
	}

	.stepper button {
		width: 2.1rem;
		height: 2.1rem;
		padding: 0;
		border: none;
		border-radius: 0;
		background: var(--input);
		font-size: 1rem;
	}

	.stepper button:hover:not(:disabled) {
		background: var(--hover);
	}

	.stepper :global(.field input) {
		width: 3rem;
		border-width: 0 1px;
		border-radius: 0;
		background: var(--surface);
		text-align: center;
	}

	.status {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--muted);
		font-size: 0.85rem;
		font-weight: 400;
	}

	.dot {
		flex: none;
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--quiet);
	}

	.live .dot {
		background: var(--ok);
	}

	.offline {
		color: var(--danger);
	}

	.offline .dot {
		background: var(--danger);
	}

	.plays,
	.notifies {
		grid-template-columns: minmax(0, 1fr);
	}

	fieldset.last {
		border-bottom: none;
	}

	.program {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.75rem;
		min-width: 0;
		cursor: pointer;
	}

	.path {
		overflow-wrap: anywhere;
	}

	.copy {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		font-weight: 500;
	}

	.link {
		padding: 0 1.25rem 0.5rem;
	}

	.link input {
		width: 100%;
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.hint.below {
		padding: 0 1.25rem 1.1rem;
	}

	/* Phones: no list, the sections alone. */
	@media (max-width: 40rem) {
		.settings {
			padding: 0.25rem 0.5rem 1rem;
		}

		nav {
			display: none;
		}
	}
</style>
