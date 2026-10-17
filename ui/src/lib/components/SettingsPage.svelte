<script lang="ts">
	import type { SettingsChange } from '@ottie/api-types/SettingsChange';
	import AppWindow from '@lucide/svelte/icons/app-window';
	import Bell from '@lucide/svelte/icons/bell';
	import ClipboardCopy from '@lucide/svelte/icons/clipboard-copy';
	import Download from '@lucide/svelte/icons/download';
	import Keyboard from '@lucide/svelte/icons/keyboard';
	import Paintbrush from '@lucide/svelte/icons/palette';
	import Play from '@lucide/svelte/icons/play';
	import Plug from '@lucide/svelte/icons/plug';
	import Power from '@lucide/svelte/icons/power';
	import RotateCw from '@lucide/svelte/icons/rotate-cw';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import { onMount } from 'svelte';
	import { prefersReducedMotion } from 'svelte/motion';
	import { appearance, setAppearance } from '#lib/appearance.svelte.js';
	import {
		COPIED_CHOICES,
		COPIED_NAMES,
		EXISTING_CHOICES,
		EXISTING_NAMES,
		NOTIFY_CHOICES,
		NOTIFY_NAMES
	} from '#lib/choices.js';
	import { useApp } from '#lib/context.js';
	import { canDo, COUNTDOWN, FINISH_CHOICES, finishHint, finishName } from '#lib/finish.js';
	import { LiveFinishing } from '#lib/finish.svelte.js';
	import { MOTIONS, type Motion } from '#lib/motion.js';
	import { resizes, shuffle, unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { HELP_TRANSLATE, REVIEWED, switchLanguage } from '#lib/languages.js';
	import { getLocale, type Locale } from '#lib/paraglide/runtime.js';
	import { RANGES, parseSeconds, parseWhole, toField } from '#lib/settings.js';
	import { lookOf, type DeviceAppearance } from '#lib/theme.js';
	import {
		canPreview,
		soundChoices,
		soundsChoices,
		type ChoiceGroup,
		type Sound,
		type SoundKind
	} from '#lib/sounds.js';
	import { STARTUP_CHOICES, STARTUP_NAMES } from '#lib/startup.js';
	import {
		canChoosePlayer,
		choosePlayer,
		desktop as desktopSetting,
		followDesktop,
		player as playerSetting,
		playSound,
		setCloseToTray,
		setCopied,
		setCountdown,
		setCountdownIn,
		setFinish,
		setFinishWaits,
		setNotify,
		setNotifyFailed,
		setPlayWith,
		setSound,
		setSounds,
		setStartup,
		setWatchClipboard,
		type Copied,
		type CountdownIn,
		type Desktop,
		type Finish,
		type Finishing,
		type Notify,
		type PlayWith,
		type Player,
		type Startup
	} from '#lib/window.js';
	import AccessSettings from './AccessSettings.svelte';
	import LanguagePicker from './LanguagePicker.svelte';
	import LimitControl from './LimitControl.svelte';
	import LookChoices from './LookChoices.svelte';
	import NumberField from './NumberField.svelte';
	import ShortcutList from './ShortcutList.svelte';
	import Stepper from './Stepper.svelte';
	import Switch from './Switch.svelte';

	const { session } = useApp();
	/** The daemon's settings; null until it has answered. */
	const settings = $derived(session.settings);

	const MOTION_NAMES: Record<Motion, () => string> = {
		system: m.set_motion_system,
		on: m.set_motion_on,
		off: m.set_motion_off
	};

	/** The choices besides a program of one's own. */
	const PLAY_NAMES: Record<'system' | 'browser', { name: () => string; hint: () => string }> = {
		system: { name: m.set_play_system, hint: m.set_play_system_hint },
		browser: { name: m.set_play_browser, hint: m.set_play_browser_hint }
	};

	/** With custom sounds, one per kind of notification (docs/decisions/0038). */
	const SOUND_NAMES: Record<SoundKind, () => string> = {
		finished: m.set_sound_finished,
		failed: m.set_sound_failed
	};
	const SOUND_KINDS = Object.keys(SOUND_NAMES) as SoundKind[];

	/** Where the countdown before sleep, hibernate or shut down shows (docs/decisions/0040). */
	const COUNTDOWN_NAMES: Record<CountdownIn, { name: () => string; hint: () => string }> = {
		window: { name: m.set_countdown_window, hint: m.set_countdown_window_hint },
		notification: { name: m.set_countdown_notification, hint: m.set_countdown_notification_hint }
	};
	const COUNTDOWN_CHOICES = Object.keys(COUNTDOWN_NAMES) as CountdownIn[];

	/** Only the desktop app can start a player (docs/decisions/0031), stay
	 * in the tray (0034), notify (0035), start at sign-in (0039) or watch
	 * the clipboard (0043). */
	const inApp = canChoosePlayer();

	/** The sections, top to bottom, for the list at the start. */
	const SECTIONS: { id: string; label: () => string }[] = [
		{ id: 'set-appearance', label: m.set_appearance },
		{ id: 'set-downloads', label: m.set_downloads },
		...(inApp
			? [
					{ id: 'set-app', label: m.set_app },
					{ id: 'set-clipboard', label: m.set_clipboard },
					{ id: 'set-finish', label: m.finish_title },
					{ id: 'set-notifications', label: m.set_notifications },
					{ id: 'set-playing', label: m.set_playing }
				]
			: []),
		{ id: 'set-retries', label: m.set_retries },
		{ id: 'set-keyboard', label: m.shortcuts_title },
		{ id: 'set-connection', label: m.set_connection }
	];

	let page: HTMLElement;
	/** The section being read: the last one whose top has scrolled past
	 * the top of the page, or the last one once at the bottom. */
	let current = $state(SECTIONS[0].id);
	let sections: HTMLElement | undefined = $state();
	// The section list's highlight slides to the section being read
	// (docs/decisions/0048).
	shuffle(
		() => sections,
		() => current
	);
	/** Why the last change wasn't saved. */
	let problem: string | null = $state(null);
	/** How the desktop app plays; null until it has answered (or in a browser). */
	let playing: Player | null = $state(null);
	/** The choice shown, put back if a change doesn't happen. */
	let playWith: PlayWith = $state('system');

	/** The desktop app's own settings; null until it has answered (or in a browser). */
	let desktop: Desktop | null = $state(null);
	/** The notify choice shown, put back if a change doesn't happen. */
	let notify: Notify = $state('all');
	/** The sign-in choice shown, put back the same way. */
	let startup: Startup = $state('off');
	/** What a copied link does, the same way. */
	let copyChoice: Copied = $state('ask');

	/** When everything has finished, as the desktop app has it. */
	const live = new LiveFinishing();
	const finishing = $derived(live.current);
	// A section whose settings arrive after it's shown grows to hold them as
	// they fade in, rather than jumping (docs/decisions/0048).
	resizes(
		() => (page ? [...page.querySelectorAll<HTMLElement>('section.card')] : undefined),
		() => [settings !== null, desktop !== null, finishing !== null, live.failed, playing !== null]
	);
	/** The choices shown, put back if a change doesn't happen. */
	let finishAlways: Finish = $derived(finishing?.always ?? 'nothing');
	let countdownIn: CountdownIn = $derived(finishing?.countdownIn ?? 'window');

	if (inApp) {
		playerSetting().then(showPlayer, () => (problem = m.set_play_failed()));
		desktopSetting().then(showDesktop, () => (problem = m.set_app_failed()));
		onMount(() => {
			live.start();
			// Also changed from the tray menu.
			const unfollow = followDesktop(showDesktop);
			return () => {
				live.stop();
				unfollow();
			};
		});
	}

	async function changeFinish(change: () => Promise<Finishing>): Promise<boolean> {
		const done = await live.change(change);
		if (!done) {
			problem = m.finish_failed();
			finishAlways = finishing?.always ?? 'nothing';
			countdownIn = finishing?.countdownIn ?? 'window';
		}
		return done;
	}

	function showDesktop(answer: Desktop) {
		desktop = answer;
		notify = answer.notify;
		startup = answer.startup;
		copyChoice = answer.copied;
	}

	async function changeDesktop(change: () => Promise<Desktop>) {
		try {
			showDesktop(await change());
		} catch {
			problem = m.set_app_failed();
			notify = desktop?.notify ?? 'all';
			startup = desktop?.startup ?? 'off';
			copyChoice = desktop?.copied ?? 'ask';
		}
	}

	function soundOf(desktop: Desktop, kind: SoundKind): Sound {
		return kind === 'finished' ? desktop.soundFinished : desktop.soundFailed;
	}

	/** A set or a sound plays as it's chosen; `saved` says what's saved. */
	async function chooseSound(
		event: Event & { currentTarget: HTMLSelectElement },
		change: (value: string) => Promise<Desktop>,
		saved: (desktop: Desktop) => string
	) {
		const select = event.currentTarget;
		const value = select.value;
		if (desktop && canPreview(value, desktop.systemSound)) {
			playSound(value);
		}
		await changeDesktop(() => change(value));
		// Shows what's saved, even if the change didn't happen.
		if (desktop) {
			select.value = saved(desktop);
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
		if (appearance.synced && !session.readOnly) {
			void save({ appearance: lookOf(appearance) });
		}
	}

	/** A language for this page, and for every device that follows (null:
	 * each its own system's). */
	async function chooseLanguage(locale: Locale | null) {
		if (appearance.synced && !session.readOnly && !(await save({ language: locale }))) {
			return;
		}
		switchLanguage(locale);
	}
</script>

{#snippet options(groups: ChoiceGroup[])}
	{#each groups as group, index (index)}
		{#if group.label}
			<optgroup label={group.label}>
				{#each group.choices as choice (choice.value)}
					<option value={choice.value}>{choice.name}</option>
				{/each}
			</optgroup>
		{:else}
			{#each group.choices as choice (choice.value)}
				<option value={choice.value}>{choice.name}</option>
			{/each}
		{/if}
	{/each}
{/snippet}

{#snippet listen(value: string, systemSound: boolean)}
	<button
		class="listen"
		disabled={!canPreview(value, systemSound)}
		aria-label={m.set_sound_play()}
		title={m.set_sound_play()}
		onclick={() => playSound(value)}
	>
		<Play size={14} aria-hidden="true" />
	</button>
{/snippet}

<div class="settings" bind:this={page} onscroll={scrolled}>
	<nav aria-label={m.set_sections()} bind:this={sections}>
		{#each SECTIONS as { id, label } (id)}
			<a
				href="#{id}"
				class:current={current === id}
				aria-current={current === id ? 'true' : undefined}
				onclick={(event) => go(event, id)}
			>
				{#if current === id}
					<span class="highlight" data-flip="highlight" data-flip-size></span>
				{/if}
				{label()}
			</a>
		{/each}
	</nav>

	<div class="sections">
		<div class="title">
			<h1>{m.set_title()}</h1>
			<!-- In this window only (docs/decisions/0050). -->
			<button onclick={() => (session.settingUpAgain = true)}>
				<Sparkles size={15} aria-hidden="true" />
				{m.set_setup_again()}
			</button>
		</div>

		{#if problem}
			<div class="problem" role="alert" in:unfold out:unfold>
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

			<LookChoices onChoose={choose} />

			<div class="setting">
				<label for="language" class="names">
					<span>{m.set_language_choice()}</span>
					<span class="hint">{m.set_language_hint()}</span>
				</label>
				<LanguagePicker id="language" onChoose={(locale) => void chooseLanguage(locale)} />
				{#if !REVIEWED.includes(getLocale())}
					<p class="hint unreviewed" in:unfold out:unfold>
						{m.language_unreviewed()}
						<a href={HELP_TRANSLATE} target="_blank" rel="noopener">{m.language_help()}</a>
					</p>
				{/if}
			</div>

			<!-- This device's alone, not part of the shared look (docs/decisions/0048). -->
			<div class="setting">
				<label for="motion" class="names">
					<span>{m.set_motion()}</span>
					<span class="hint">
						{appearance.motion === 'system' && prefersReducedMotion.current
							? m.set_motion_system_less()
							: m.set_motion_hint()}
					</span>
				</label>
				<select
					id="motion"
					class="motion"
					value={appearance.motion}
					onchange={(event) => {
						const chosen = MOTIONS.find((motion) => motion === event.currentTarget.value);
						if (chosen) {
							setAppearance({ motion: chosen });
						}
					}}
				>
					{#each MOTIONS as motion (motion)}
						<option value={motion}>{MOTION_NAMES[motion]()}</option>
					{/each}
				</select>
			</div>
		</section>

		<section id="set-downloads" class="card" aria-labelledby="downloads-title">
			<header>
				<span class="icon" aria-hidden="true"><Download size={16} /></span>
				<h2 id="downloads-title">{m.set_downloads()}</h2>
			</header>
			{#if settings}
				<fieldset class="shared" disabled={session.readOnly}>
					<div class="setting">
						<span class="names">
							<span>{m.set_max_concurrent()}</span>
							<span class="hint">{m.set_max_concurrent_hint()}</span>
						</span>
						<Stepper
							value={settings.maxConcurrent}
							range={RANGES.maxConcurrent}
							label={m.set_max_concurrent()}
							onChange={(maxConcurrent) => save({ maxConcurrent })}
						/>
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
					<div class="setting">
						<label for="existing" class="names">
							<span>{m.set_existing()}</span>
							<span class="hint">{m.set_existing_hint()}</span>
						</label>
						<select
							id="existing"
							class="existing"
							value={settings.existing}
							onchange={(event) => {
								const value = event.currentTarget.value;
								const chosen = EXISTING_CHOICES.find((existing) => existing === value);
								if (chosen) {
									void save({ existing: chosen });
								}
							}}
						>
							{#each EXISTING_CHOICES as existing (existing)}
								<option value={existing}>{EXISTING_NAMES[existing]()}</option>
							{/each}
						</select>
					</div>
					<div class="setting">
						<span class="names">
							<span>{m.set_max_batch()}</span>
							<span class="hint">{m.set_max_batch_hint()}</span>
						</span>
						<NumberField
							value={settings.maxBatch}
							label={m.set_max_batch()}
							parse={(text) => parseWhole(text, RANGES.maxBatch)}
							onChange={(maxBatch) => save({ maxBatch })}
						/>
					</div>
				</fieldset>
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
					<fieldset>
						<legend>{m.set_startup()}</legend>
						<div class="options startups">
							{#each STARTUP_CHOICES as choice (choice)}
								<label class="option row" class:chosen={startup === choice}>
									<input
										type="radio"
										name="startup"
										value={choice}
										bind:group={startup}
										onchange={() => changeDesktop(() => setStartup(choice))}
									/>
									<span class="names">
										<span>{STARTUP_NAMES[choice].name()}</span>
										<span class="hint">{STARTUP_NAMES[choice].hint()}</span>
									</span>
								</label>
							{/each}
						</div>
					</fieldset>
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

			<section id="set-clipboard" class="card" aria-labelledby="clipboard-title">
				<header>
					<span class="icon" aria-hidden="true"><ClipboardCopy size={16} /></span>
					<h2 id="clipboard-title">{m.set_clipboard()}</h2>
					<p class="hint">{m.set_clipboard_hint()}</p>
				</header>
				{#if desktop}
					<div class="setting">
						<span class="names">
							<span>{m.set_watch_clipboard()}</span>
							<span class="hint">{m.set_watch_clipboard_hint()}</span>
						</span>
						<Switch
							on={desktop.watchClipboard}
							label={m.set_watch_clipboard()}
							onChange={(on) => changeDesktop(() => setWatchClipboard(on))}
						/>
					</div>
					<fieldset class="last" disabled={!desktop.watchClipboard}>
						<legend>{m.set_copied()}</legend>
						<div class="options copies">
							{#each COPIED_CHOICES as choice (choice)}
								<label class="option row" class:chosen={copyChoice === choice}>
									<input
										type="radio"
										name="copied"
										value={choice}
										bind:group={copyChoice}
										onchange={() => changeDesktop(() => setCopied(choice))}
									/>
									<span class="names">
										<span>{COPIED_NAMES[choice].name()}</span>
										<span class="hint">{COPIED_NAMES[choice].hint()}</span>
									</span>
								</label>
							{/each}
						</div>
					</fieldset>
				{:else}
					<p class="waiting">{m.connection_connecting()}</p>
				{/if}
			</section>

			<section id="set-finish" class="card" aria-labelledby="finish-title">
				<header>
					<span class="icon" aria-hidden="true"><Power size={16} /></span>
					<h2 id="finish-title">{m.finish_title()}</h2>
					<p class="hint">{m.set_finish_hint()}</p>
				</header>
				{#if finishing}
					<fieldset>
						<legend>{m.set_finish_always()}</legend>
						<div class="options finishes">
							{#each FINISH_CHOICES as choice (choice)}
								{@const can = canDo(finishing, choice)}
								<label
									class="option row"
									class:chosen={finishAlways === choice}
									class:unable={!can}
								>
									<input
										type="radio"
										name="finish"
										value={choice}
										disabled={!can}
										bind:group={finishAlways}
										onchange={() => changeFinish(() => setFinish(choice))}
									/>
									<span class="names">
										<span>{finishName(choice)}</span>
										<span class="hint">{finishHint(finishing, choice)}</span>
									</span>
								</label>
							{/each}
						</div>
					</fieldset>
					<div class="setting">
						<span class="names">
							<span>{m.set_countdown()}</span>
							<span class="hint">{m.set_countdown_hint()}</span>
						</span>
						<NumberField
							value={finishing.countdown}
							label={m.set_countdown()}
							unit={m.unit_seconds()}
							parse={(text) => parseWhole(text, COUNTDOWN)}
							onChange={(seconds) => changeFinish(() => setCountdown(seconds))}
						/>
					</div>
					<fieldset>
						<legend>{m.set_countdown_in()}</legend>
						<div class="options countdowns">
							{#each COUNTDOWN_CHOICES as choice (choice)}
								<label class="option row" class:chosen={countdownIn === choice}>
									<input
										type="radio"
										name="countdown-in"
										value={choice}
										bind:group={countdownIn}
										onchange={() => changeFinish(() => setCountdownIn(choice))}
									/>
									<span class="names">
										<span>{COUNTDOWN_NAMES[choice].name()}</span>
										<span class="hint">{COUNTDOWN_NAMES[choice].hint()}</span>
									</span>
								</label>
							{/each}
						</div>
					</fieldset>
					<div class="setting">
						<span class="names">
							<span>{m.set_finish_waits()}</span>
							<span class="hint">{m.set_finish_waits_hint()}</span>
						</span>
						<Switch
							on={finishing.waitsForScheduled}
							label={m.set_finish_waits()}
							onChange={(on) => changeFinish(() => setFinishWaits(on))}
						/>
					</div>
				{:else if live.failed}
					<p class="waiting">{m.finish_failed()}</p>
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
					<div class="setting">
						<label for="sounds" class="names">
							<span>{m.set_sounds()}</span>
							<span class="hint">{m.set_sounds_hint()}</span>
						</label>
						<span class="sound">
							<select
								id="sounds"
								value={desktop.sounds}
								onchange={(event) => chooseSound(event, setSounds, (saved) => saved.sounds)}
							>
								{@render options(soundsChoices(desktop.sets))}
							</select>
							{@render listen(desktop.sounds, desktop.systemSound)}
						</span>
					</div>
					{#if desktop.sounds === 'custom'}
						{#each SOUND_KINDS as kind (kind)}
							{@const sound = soundOf(desktop, kind)}
							<div class="setting" in:unfold out:unfold>
								<label for="sound-{kind}" class="names">
									<span>{SOUND_NAMES[kind]()}</span>
								</label>
								<span class="sound">
									<select
										id="sound-{kind}"
										value={sound}
										onchange={(event) =>
											chooseSound(
												event,
												(value) => setSound(kind, value),
												(saved) => soundOf(saved, kind)
											)}
									>
										{@render options(soundChoices(desktop.sets, kind))}
									</select>
									{@render listen(sound, desktop.systemSound)}
								</span>
							</div>
						{/each}
					{/if}
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
				<fieldset class="shared" disabled={session.readOnly}>
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
				</fieldset>
			{:else}
				<p class="waiting">{m.connection_connecting()}</p>
			{/if}
		</section>

		<section id="set-keyboard" class="card" aria-labelledby="keyboard-title">
			<header>
				<span class="icon" aria-hidden="true"><Keyboard size={16} /></span>
				<h2 id="keyboard-title">{m.shortcuts_title()}</h2>
				<p class="hint">{m.set_keyboard_hint()}</p>
			</header>
			<div class="shortcuts"><ShortcutList /></div>
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
			</div>
			<AccessSettings />
		</section>
	</div>
</div>

<style>
	/* The section list at the start, the sections taking the rest. */
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
		/* Its own layer: the chosen one's highlight slides under every
		   item's text (each item isolated, it would cover the earlier ones). */
		isolation: isolate;
		position: sticky;
		top: 0;
		display: grid;
		flex: 0 0 9.5rem;
		gap: 0.15rem;
		padding-top: 3.1rem;
	}

	nav a {
		position: relative;
		padding: 0.45rem 0.75rem;
		color: var(--muted);
		text-decoration: none;
		border-radius: 999px;
	}

	nav a:not(.current):hover {
		background: var(--hover);
		color: var(--text);
	}

	nav a.current {
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

	/* The page's title, and the setup run again at the end of its row. */
	.title {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.title button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.problem {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.6rem 0.9rem;
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
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
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

	/* The card's title and hint, and a switch for all of it at the end. */
	header.with-switch {
		column-gap: 1rem;
	}

	header.with-switch .icon {
		margin-inline-end: -0.25rem;
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

	.option input {
		margin: 0;
		accent-color: var(--accent);
	}

	.names {
		display: grid;
		flex: 1 1 16rem;
		font-weight: 500;
	}

	.names .hint {
		font-weight: 400;
	}

	/* One setting per row: its name and hint, then the control. */
	.shortcuts {
		padding: 0.9rem 1.25rem 1.1rem;
	}

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

	/* Under the language, across the row. */
	.unreviewed {
		flex-basis: 100%;
	}

	.unreviewed a {
		color: var(--accent);
		font-weight: 500;
		text-decoration: none;
	}

	.unreviewed a:hover {
		text-decoration: underline;
	}

	.sound {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.motion,
	.existing {
		min-width: 10rem;
		padding: 0.35rem 0.6rem;
	}

	.sound select {
		min-width: 10rem;
		padding: 0.35rem 0.6rem;
	}

	.listen {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
	}

	.waiting {
		margin: 0;
		padding: 0.5rem 1.25rem 1.1rem;
		color: var(--faint);
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
	.notifies,
	.startups,
	.finishes,
	.countdowns,
	.copies {
		grid-template-columns: minmax(0, 1fr);
	}

	/* A choice this computer can't make: listed, greyed out, with why. Or
	   one that applies only once a switch above is on. */
	fieldset:disabled .option,
	.option.unable {
		opacity: 0.55;
		cursor: default;
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

	/* A read-only device sees the shared settings but can't change them. */
	.shared {
		display: contents;
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
		border-bottom-color: var(--border-soft);
		background: none;
	}

	.problem {
		border-radius: var(--radius-m);
	}
</style>
