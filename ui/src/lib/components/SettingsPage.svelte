<script lang="ts">
	import { appearance, setAppearance } from '#lib/appearance.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { MODES, PALETTES, THEMES, previewFor, type Mode, type Palette } from '#lib/theme.js';
	import LanguagePicker from './LanguagePicker.svelte';

	const MODE_NAMES: Record<Mode, () => string> = {
		system: m.set_mode_system,
		light: m.set_mode_light,
		dark: m.set_mode_dark
	};

	const PALETTE_NAMES: Record<Palette, { name: () => string; hint: () => string }> = {
		sea: { name: m.set_palette_sea, hint: m.set_palette_sea_hint },
		neutral: { name: m.set_palette_neutral, hint: m.set_palette_neutral_hint }
	};

	/** The sections, top to bottom, for the list on the left. */
	const SECTIONS: { id: string; label: () => string }[] = [
		{ id: 'set-appearance', label: m.set_appearance },
		{ id: 'set-language', label: m.language }
	];

	let page: HTMLElement;
	/** The section being read: the last one whose top has scrolled past
	 * the top of the page, or the last one once at the bottom. */
	let current = $state(SECTIONS[0].id);

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

		<section id="set-appearance" class="card" aria-labelledby="appearance">
			<header>
				<h2 id="appearance">{m.set_appearance()}</h2>
				<p class="hint">{m.set_appearance_hint()}</p>
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
									onchange={() => setAppearance({ mode })}
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
								onchange={() => setAppearance({ palette })}
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
		</section>

		<section id="set-language" class="card" aria-labelledby="language-title">
			<header>
				<h2 id="language-title">{m.language()}</h2>
			</header>
			<div class="setting">
				<label for="language" class="names">
					<span>{m.set_language_choice()}</span>
					<span class="hint">{m.set_language_hint()}</span>
				</label>
				<LanguagePicker id="language" />
			</div>
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

	.card {
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
	}

	header {
		padding: 1rem 1.25rem 0.25rem;
	}

	h2 {
		margin: 0;
		font-size: 1.05rem;
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

	fieldset:last-child {
		border-bottom: none;
	}

	legend {
		float: left;
		margin-bottom: 0.6rem;
		padding: 0;
		font-weight: 500;
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
		flex: 1;
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

	.setting {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 1rem;
		padding: 0.9rem 1.25rem 1.1rem;
	}

	.setting label {
		flex: 1 1 16rem;
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
