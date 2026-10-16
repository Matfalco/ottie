<script lang="ts">
	import { appearance } from '#lib/appearance.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		MODES,
		PALETTES,
		THEMES,
		previewFor,
		type DeviceAppearance,
		type Mode,
		type Palette
	} from '#lib/theme.js';

	interface Props {
		/** Applies a choice (here, and for the devices that follow it). */
		onChoose: (change: Partial<DeviceAppearance>) => void;
	}

	let { onChoose }: Props = $props();

	const MODE_NAMES: Record<Mode, () => string> = {
		system: m.set_mode_system,
		light: m.set_mode_light,
		dark: m.set_mode_dark
	};

	const PALETTE_NAMES: Record<Palette, { name: () => string; hint: () => string }> = {
		sea: { name: m.set_palette_sea, hint: m.set_palette_sea_hint },
		neutral: { name: m.set_palette_neutral, hint: m.set_palette_neutral_hint }
	};
</script>

<fieldset>
	<legend>{m.set_mode()}</legend>
	<div class="options modes">
		{#each MODES as mode (mode)}
			{@const [left, right] = previewFor(mode, appearance.palette)}
			<label class="option" class:chosen={appearance.mode === mode}>
				<!-- Each half carries its theme, so it shows that theme's colours. -->
				<span class="preview" aria-hidden="true">
					<span class="half" data-theme={left}>
						<span class="bar"></span><span class="line"></span><span class="line short"></span>
					</span>
					<span class="half" data-theme={right}>
						<span class="bar"></span><span class="line"></span><span class="line short"></span>
					</span>
				</span>
				<span class="choice">
					<input
						type="radio"
						name="mode"
						checked={appearance.mode === mode}
						onchange={() => onChoose({ mode })}
					/>
					{MODE_NAMES[mode]()}
				</span>
			</label>
		{/each}
	</div>
</fieldset>

<!-- The setting after it draws the line between them. -->
<fieldset class="last">
	<legend>{m.set_palette()}</legend>
	<div class="options palettes">
		{#each PALETTES as palette (palette)}
			<label class="option row" class:chosen={appearance.palette === palette}>
				<input
					type="radio"
					name="palette"
					checked={appearance.palette === palette}
					onchange={() => onChoose({ palette })}
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

<style>
	fieldset {
		margin: 0;
		padding: 0.75rem 1.25rem 1rem;
		border: none;
		border-bottom: 1px solid var(--border-soft);
	}

	fieldset.last {
		border-bottom: none;
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

	input {
		margin: 0;
		accent-color: var(--accent);
	}

	.preview {
		display: flex;
		height: 4rem;
		overflow: hidden;
		border: 1px solid var(--border-soft);
		border-radius: var(--radius-m);
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

	.hint {
		color: var(--faint);
		font-size: 0.85rem;
		font-weight: 400;
	}

	/* Each palette's light and dark theme: its background with its accent. */
	.swatches {
		display: flex;
	}

	.swatches span {
		width: 1.9rem;
		height: 1.9rem;
		border: 2px solid var(--surface);
		border-radius: 50%;
		background: linear-gradient(135deg, var(--bg) 50%, var(--accent) 50%);
	}

	.swatches span + span {
		margin-inline-start: -0.6rem;
	}
</style>
