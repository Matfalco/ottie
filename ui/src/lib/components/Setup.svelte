<script lang="ts">
	import type { CategoryBody } from '@ottie/api-types/CategoryBody';
	import type { Existing } from '@ottie/api-types/Existing';
	import type { SetupStep } from '@ottie/api-types/SetupStep';
	import Check from '@lucide/svelte/icons/check';
	import Folder from '@lucide/svelte/icons/folder';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Play from '@lucide/svelte/icons/play';
	import Plus from '@lucide/svelte/icons/plus';
	import icon from '#lib/assets/ottie.svg';
	import { appearance, setAppearance } from '#lib/appearance.svelte.js';
	import { GENERAL } from '#lib/categories.js';
	import {
		COPIED_CHOICES,
		COPIED_NAMES,
		EXISTING_CHOICES,
		EXISTING_NAMES,
		NOTIFY_CHOICES,
		NOTIFY_NAMES
	} from '#lib/choices.js';
	import { useApp } from '#lib/context.js';
	import { switchLanguage } from '#lib/languages.js';
	import { appear, unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Locale } from '#lib/paraglide/runtime.js';
	import { RANGES } from '#lib/settings.js';
	import {
		asCategory,
		edited,
		folderOf,
		notMade,
		startAt,
		starters,
		stepsHere,
		toCreate,
		type Starter
	} from '#lib/setup.js';
	import { canPreview, soundsChoices } from '#lib/sounds.js';
	import { FIRST_STARTUP, STARTUP_CHOICES, STARTUP_NAMES } from '#lib/startup.js';
	import { lookOf, type DeviceAppearance } from '#lib/theme.js';
	import {
		canPickFolder,
		canStartAtSignIn,
		desktop as desktopSetting,
		pickFolder,
		playSound,
		setCloseToTray,
		setCopied,
		setNotify,
		setNotifyFailed,
		setSounds,
		setStartup,
		setWatchClipboard,
		type Copied,
		type Desktop,
		type Notify,
		type Startup
	} from '#lib/window.js';
	import CategoryEditor from './CategoryEditor.svelte';
	import LanguagePicker from './LanguagePicker.svelte';
	import LookChoices from './LookChoices.svelte';
	import Stepper from './Stepper.svelte';
	import Switch from './Switch.svelte';

	const { session } = useApp();

	/** Run again from Settings, rather than the first start. */
	const again = session.settingUpAgain;
	/** The desktop app has steps of its own (docs/decisions/0050). */
	const inApp = canStartAtSignIn();
	const steps = stepsHere(inApp);
	// The settings arrive with the categories, before this page shows.
	let at = $state(again ? 0 : startAt(steps, session.settings?.setupStep ?? null));
	const step = $derived(steps[at]);
	const last = $derived(at === steps.length - 1);
	let saving = $state(false);
	let problem: string | null = $state(null);

	const STEP_NAMES: Record<SetupStep, () => string> = {
		look: m.setup_step_look,
		downloads: m.set_downloads,
		categories: m.cat_title,
		startup: m.setup_step_startup,
		clipboard: m.set_clipboard,
		notifications: m.set_notifications
	};

	// The downloads step.
	let downloads = $state(session.categories[GENERAL]?.folder ?? '');
	let maxConcurrent = $state(session.settings?.maxConcurrent ?? 3);
	let existing: Existing = $state(session.settings?.existing ?? 'rename');

	// The categories step: in General's folder as saved (the downloads step
	// may have been skipped).
	const generalFolder = $derived(session.categories[GENERAL]?.folder ?? '');
	let list: Starter[] = $state(notMade(starters(), session.categoryList));
	/** The row in the editor: `null` for a new one, `undefined` when closed. */
	let editing: Starter | null | undefined = $state(undefined);
	let added = 0;

	// The desktop app's steps, as the app has them (null until it answers);
	// sign-in starts the tray at first, unless chosen otherwise.
	let desktop: Desktop | null = $state(null);
	let startup: Startup = $state(FIRST_STARTUP);
	let closeToTray = $state(true);
	let watchClipboard = $state(false);
	let copied: Copied = $state('ask');
	let notify: Notify = $state('all');
	let notifyFailed = $state(true);
	let sounds = $state('system');

	if (inApp) {
		desktopSetting().then(
			(answer) => {
				desktop = answer;
				startup = again ? answer.startup : FIRST_STARTUP;
				closeToTray = answer.closeToTray;
				watchClipboard = answer.watchClipboard;
				copied = answer.copied;
				notify = answer.notify;
				notifyFailed = answer.notifyFailed;
				sounds = answer.sounds;
			},
			() => (problem = m.set_app_failed())
		);
	}

	/** Changes the look here, and for every device that follows it. */
	function chooseLook(change: Partial<DeviceAppearance>) {
		setAppearance(change);
		if (appearance.synced) {
			void session.changeSettings({ appearance: lookOf(appearance) });
		}
	}

	/** A language for this page, and for every device that follows. The page
	 * reloads in it, on this step: it isn't passed yet. */
	async function chooseLanguage(locale: Locale | null) {
		if (appearance.synced) {
			problem = await session.changeSettings({ language: locale });
			if (problem !== null) {
				return;
			}
		}
		switchLanguage(locale);
	}

	async function browse() {
		const chosen = await pickFolder(downloads);
		if (chosen) {
			downloads = chosen;
		}
	}

	async function saveStarter(body: CategoryBody): Promise<string | null> {
		if (editing) {
			const key = editing.key;
			const index = list.findIndex((starter) => starter.key === key);
			list[index] = edited(editing, body, generalFolder);
		} else {
			added += 1;
			const fresh: Starter = { key: `added-${added}`, body, chosen: true, folderFollows: false };
			list.push(edited(fresh, body, generalFolder));
		}
		return null;
	}

	/** The desktop app's settings, one after the other; null once saved,
	 * else why not. */
	async function changeDesktop(...changes: (() => Promise<Desktop>)[]): Promise<string | null> {
		if (desktop === null) {
			return null; // The app never answered: nothing was shown to save.
		}
		try {
			for (const change of changes) {
				desktop = await change();
			}
			return null;
		} catch {
			return m.set_app_failed();
		}
	}

	/** Saves what the step shows, then goes past it; null once done, else
	 * why not (what was saved stays). */
	async function apply(): Promise<string | null> {
		switch (step) {
			case 'look':
				// Already applied as chosen.
				return session.passStep(step);
			case 'downloads': {
				const folder = downloads.trim();
				const why = folder ? await session.setDownloadsFolder(folder) : null;
				return why ?? session.passStep(step, { maxConcurrent, existing });
			}
			case 'categories': {
				const why = await session.addCategories(
					toCreate(list, generalFolder, session.categoryList)
				);
				if (why !== null) {
					return why;
				}
				// Those made aren't offered again if one comes back here.
				list = list.filter((starter) => !starter.chosen);
				return session.passStep(step);
			}
			case 'startup': {
				const why = await changeDesktop(
					() => setStartup(startup),
					() => setCloseToTray(closeToTray)
				);
				return why ?? session.passStep(step);
			}
			case 'clipboard': {
				const why = await changeDesktop(
					() => setWatchClipboard(watchClipboard),
					() => setCopied(copied)
				);
				return why ?? session.passStep(step);
			}
			case 'notifications': {
				const why = await changeDesktop(
					() => setNotify(notify),
					() => setNotifyFailed(notifyFailed),
					() => setSounds(sounds)
				);
				return why ?? session.passStep(step);
			}
		}
	}

	/** Next (saving the step) or Skip (leaving things as they were). */
	async function next(saveIt: boolean) {
		saving = true;
		problem = null;
		let why = saveIt ? await apply() : await session.passStep(step);
		if (why === null && last) {
			why = await session.endSetup();
		}
		saving = false;
		if (why !== null) {
			problem = why;
		} else if (!last) {
			at += 1;
		}
	}

	function back() {
		problem = null;
		at -= 1;
	}

	/** Ends it here; the steps already passed stay saved. */
	async function skipAll() {
		saving = true;
		problem = await session.endSetup();
		saving = false;
	}
</script>

<div class="page">
	<div class="content">
		<header class="welcome">
			<img src={icon} alt="" width="48" height="48" />
			<div>
				<h1>{m.setup_title()}</h1>
				<p>{m.setup_welcome()}</p>
			</div>
		</header>

		<ol class="steps" aria-label={m.setup_steps()}>
			{#each steps as id, index (id)}
				<li
					class:done={index < at}
					class:current={index === at}
					aria-current={index === at ? 'step' : undefined}
				>
					<span class="dot" aria-hidden="true">
						{#if index < at}<Check size={13} />{:else}{index + 1}{/if}
					</span>
					<span class="visually-hidden">{STEP_NAMES[id]()}</span>
				</li>
			{/each}
		</ol>

		{#key step}
			<section class="card" aria-labelledby="step-title" in:appear>
				<header>
					<span class="progress">{m.setup_progress({ current: at + 1, total: steps.length })}</span>
					<h2 id="step-title">{STEP_NAMES[step]()}</h2>
				</header>

				{#if step === 'look'}
					<p class="intro">{m.setup_look_intro()}</p>
					<div class="setting">
						<label for="language" class="names">
							<span>{m.set_language_choice()}</span>
							<span class="hint">{m.set_language_hint()}</span>
						</label>
						<LanguagePicker id="language" onChoose={(locale) => void chooseLanguage(locale)} />
					</div>
					<LookChoices onChoose={chooseLook} />
				{:else if step === 'downloads'}
					<p class="intro">{m.setup_downloads_intro()}</p>
					<div class="setting">
						<label for="downloads" class="names">
							<span>{m.setup_folder()}</span>
							<span class="hint">{m.setup_folder_hint()}</span>
						</label>
						<span class="folder-field">
							<input id="downloads" bind:value={downloads} dir="auto" />
							{#if canPickFolder()}
								<button type="button" onclick={browse}>
									<FolderOpen size={15} aria-hidden="true" />
									{m.cat_browse()}
								</button>
							{/if}
						</span>
					</div>
					<div class="setting">
						<span class="names">
							<span>{m.set_max_concurrent()}</span>
							<span class="hint">{m.set_max_concurrent_hint()}</span>
						</span>
						<Stepper
							value={maxConcurrent}
							range={RANGES.maxConcurrent}
							label={m.set_max_concurrent()}
							onChange={async (value) => {
								maxConcurrent = value;
								return true;
							}}
						/>
					</div>
					<fieldset>
						<legend>{m.set_existing()}</legend>
						<p class="hint legend-hint">{m.set_existing_hint()}</p>
						<div class="options">
							{#each EXISTING_CHOICES as choice (choice)}
								<label class="option" class:chosen={existing === choice}>
									<input type="radio" name="existing" value={choice} bind:group={existing} />
									<span class="names"><span>{EXISTING_NAMES[choice]()}</span></span>
								</label>
							{/each}
						</div>
					</fieldset>
				{:else if step === 'categories'}
					<p class="intro">{m.setup_intro()}</p>
					{#if list.length}
						<ul class="starters">
							{#each list as starter (starter.key)}
								{@const folder = folderOf(starter, generalFolder)}
								<li class:off={!starter.chosen}>
									<input
										type="checkbox"
										bind:checked={starter.chosen}
										aria-labelledby="starter-{starter.key}"
									/>
									<div class="about">
										<span class="name" id="starter-{starter.key}">{starter.body.name}</span>
										<span class="extensions" title={starter.body.extensions.join(', ')}>
											{starter.body.extensions.join(', ') || m.cat_no_extensions()}
										</span>
										<span class="folder" title={folder ?? undefined}>
											<Folder size={13} aria-hidden="true" />
											<span dir="auto">{folder ?? m.cat_no_folder()}</span>
										</span>
									</div>
									<button
										class="edit"
										onclick={() => (editing = starter)}
										aria-label={m.cat_edit_label({ name: starter.body.name })}
										title={m.cat_edit()}
									>
										<Pencil size={15} />
									</button>
								</li>
							{/each}
						</ul>
					{:else}
						<p class="intro">{m.setup_categories_made()}</p>
					{/if}
					<div class="add-row">
						<button onclick={() => (editing = null)}>
							<Plus size={16} aria-hidden="true" />
							{m.setup_add()}
						</button>
					</div>
				{:else if desktop === null}
					<p class="intro">{m.connection_connecting()}</p>
				{:else if step === 'startup'}
					<p class="intro">{m.set_app_hint()}</p>
					<fieldset>
						<legend>{m.set_startup()}</legend>
						<div class="options">
							{#each STARTUP_CHOICES as choice (choice)}
								<label class="option" class:chosen={startup === choice}>
									<input type="radio" name="startup" value={choice} bind:group={startup} />
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
							on={closeToTray}
							label={m.set_close_to_tray()}
							onChange={(on) => (closeToTray = on)}
						/>
					</div>
				{:else if step === 'clipboard'}
					<p class="intro">{m.setup_clipboard_hint()}</p>
					<div class="setting">
						<span class="names">
							<span>{m.set_watch_clipboard()}</span>
							<span class="hint">{m.set_watch_clipboard_hint()}</span>
						</span>
						<Switch
							on={watchClipboard}
							label={m.set_watch_clipboard()}
							onChange={(on) => (watchClipboard = on)}
						/>
					</div>
					{#if watchClipboard}
						<fieldset in:unfold out:unfold>
							<legend>{m.set_copied()}</legend>
							<div class="options">
								{#each COPIED_CHOICES as choice (choice)}
									<label class="option" class:chosen={copied === choice}>
										<input type="radio" name="copied" value={choice} bind:group={copied} />
										<span class="names">
											<span>{COPIED_NAMES[choice].name()}</span>
											<span class="hint">{COPIED_NAMES[choice].hint()}</span>
										</span>
									</label>
								{/each}
							</div>
						</fieldset>
					{/if}
				{:else if step === 'notifications'}
					<p class="intro">{m.set_notifications_hint()}</p>
					<fieldset>
						<legend>{m.set_notify()}</legend>
						<div class="options">
							{#each NOTIFY_CHOICES as choice (choice)}
								<label class="option" class:chosen={notify === choice}>
									<input type="radio" name="notify" value={choice} bind:group={notify} />
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
							on={notifyFailed}
							label={m.set_notify_failed()}
							onChange={(on) => (notifyFailed = on)}
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
								bind:value={sounds}
								onchange={() => {
									if (desktop && canPreview(sounds, desktop.systemSound)) {
										playSound(sounds);
									}
								}}
							>
								{#each soundsChoices(desktop.sets) as group, index (index)}
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
							</select>
							<button
								class="listen"
								disabled={!canPreview(sounds, desktop.systemSound)}
								aria-label={m.set_sound_play()}
								title={m.set_sound_play()}
								onclick={() => playSound(sounds)}
							>
								<Play size={14} aria-hidden="true" />
							</button>
						</span>
					</div>
				{/if}
			</section>
		{/key}

		{#if problem}
			<p class="problem" role="alert" in:unfold out:unfold>{problem}</p>
		{/if}

		<div class="buttons">
			<button class="quiet" disabled={saving} onclick={skipAll}>
				{again ? m.setup_close() : m.setup_skip_all()}
			</button>
			<span class="moves">
				{#if at > 0}
					<button disabled={saving} onclick={back}>{m.setup_back()}</button>
				{/if}
				<button disabled={saving} onclick={() => next(false)}>{m.setup_skip_step()}</button>
				<button class="primary" disabled={saving} onclick={() => next(true)}>
					{#if !last}
						{m.setup_next()}
					{:else if again}
						{m.setup_done()}
					{:else}
						{m.setup_finish()}
					{/if}
				</button>
			</span>
		</div>
	</div>
</div>

{#if editing !== undefined}
	<CategoryEditor
		category={editing ? asCategory(editing, generalFolder) : null}
		generalFolder={generalFolder.trim() || null}
		onSave={saveStarter}
		onClose={() => (editing = undefined)}
	/>
{/if}

<style>
	.page {
		overflow: auto;
		background: var(--bg);
	}

	.content {
		display: grid;
		gap: 1.25rem;
		max-width: 44rem;
		margin: 0 auto;
		padding: 2rem 1.25rem;
	}

	.welcome {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
	}

	h1 {
		margin: 0;
		font-size: 1.5rem;
	}

	.welcome p {
		margin: 0.4rem 0 0;
		color: var(--muted);
	}

	/* The step bar: a numbered dot per step, joined by a line. */
	.steps {
		display: flex;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.steps li {
		position: relative;
		display: flex;
		flex: 1;
		align-items: center;
	}

	.steps li:last-child {
		flex: none;
	}

	.steps li:not(:last-child)::after {
		content: '';
		flex: 1;
		height: 2px;
		margin: 0 0.4rem;
		border-radius: 1px;
		background: var(--border);
	}

	.steps li.done::after {
		background: var(--accent);
	}

	.dot {
		display: grid;
		flex: none;
		place-items: center;
		width: 1.75rem;
		height: 1.75rem;
		border: 2px solid var(--border);
		border-radius: 50%;
		background: var(--surface);
		color: var(--muted);
		font-size: 0.8rem;
		font-weight: 600;
	}

	.current .dot {
		border-color: var(--accent);
		color: var(--accent);
	}

	.done .dot {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--on-accent);
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	.card {
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	.card > header {
		display: grid;
		gap: 0.15rem;
		padding: 0.8rem 1.25rem;
		border-bottom: 1px solid var(--border-soft);
	}

	h2 {
		margin: 0;
		font-size: 1.15rem;
		font-weight: 600;
	}

	.progress {
		color: var(--faint);
		font-size: 0.8rem;
	}

	.intro {
		margin: 0;
		padding: 0.8rem 1.25rem 0;
		color: var(--muted);
	}

	.hint {
		margin: 0;
		color: var(--faint);
		font-size: 0.85rem;
		font-weight: 400;
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

	.intro + .setting {
		border-top: none;
	}

	.names {
		display: grid;
		flex: 1 1 16rem;
		font-weight: 500;
	}

	.folder-field {
		display: flex;
		flex: 1 1 18rem;
		gap: 0.4rem;
	}

	.folder-field input {
		flex: 1 1 0;
		width: 0;
		min-width: 0;
	}

	button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		white-space: nowrap;
	}

	fieldset {
		margin: 0;
		padding: 0.75rem 1.25rem 1rem;
		border: none;
		border-top: 1px solid var(--border-soft);
	}

	.intro + fieldset {
		border-top: none;
	}

	legend {
		float: left;
		margin-bottom: 0.6rem;
		padding: 0;
		color: var(--muted);
		font-size: 0.85rem;
		font-weight: 600;
	}

	.legend-hint {
		clear: both;
		margin-bottom: 0.6rem;
	}

	.options {
		display: grid;
		clear: both;
		gap: 0.65rem;
	}

	.option {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 0.75rem;
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

	.option input {
		margin: 0;
		accent-color: var(--accent);
	}

	.starters {
		margin: 0.8rem 1.25rem 0;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		list-style: none;
	}

	.starters li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 0.75rem;
	}

	.starters li + li {
		border-top: 1px solid var(--border-soft);
	}

	.starters li.off .about {
		opacity: 0.5;
	}

	input[type='checkbox'] {
		flex: none;
		width: 1.05rem;
		height: 1.05rem;
		accent-color: var(--accent);
	}

	.about {
		display: grid;
		grid-template-columns: 9rem minmax(0, 1fr);
		gap: 0.15rem 0.75rem;
		flex: 1;
		min-width: 0;
	}

	.name {
		grid-row: span 2;
		align-self: center;
		font-weight: 600;
		overflow-wrap: anywhere;
	}

	.extensions,
	.folder span {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.extensions {
		font-size: 0.85rem;
	}

	.folder {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		min-width: 0;
		color: var(--muted);
		font-size: 0.8rem;
	}

	.folder :global(svg) {
		flex: none;
	}

	.edit {
		display: grid;
		flex: none;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.edit:hover {
		background: var(--hover);
		color: var(--text);
	}

	.add-row {
		display: flex;
		padding: 0.8rem 1.25rem 1rem;
	}

	.sound {
		display: flex;
		align-items: center;
		gap: 0.4rem;
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

	.problem {
		margin: 0;
		color: var(--danger);
	}

	/* Skip the setup at the start; Back, Skip and Next at the end. */
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.moves {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-inline-start: auto;
	}

	.quiet {
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.quiet:hover:not(:disabled) {
		background: var(--hover);
		color: var(--text);
	}

	@media (max-width: 30rem) {
		.content {
			padding: 1.25rem 0.75rem;
		}

		.about {
			grid-template-columns: minmax(0, 1fr);
		}

		.name {
			grid-row: auto;
		}
	}
</style>
