<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import type { CategoryBody } from '@ottie/api-types/CategoryBody';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import { categoryName, parseExtensions, suggestFolder } from '#lib/categories.js';
	import { unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import {
		SPEED_UNITS,
		fieldToLimit,
		limitToField,
		unitLabel,
		type SpeedUnit
	} from '#lib/settings.js';
	import { canPickFolder, pickFolder } from '#lib/window.js';

	interface Props {
		/** The category to edit; null for a new one. */
		category: Category | null;
		/** General's folder, to suggest a subfolder for a new category. */
		generalFolder: string | null;
		/** Resolves to null once saved, else to why not. */
		onSave: (body: CategoryBody) => Promise<string | null>;
		onClose: () => void;
	}

	let { category, generalFolder, onSave, onClose }: Props = $props();
	/** Other has only a name and limits: its downloads chose their folder. */
	// svelte-ignore state_referenced_locally
	const other = category?.builtIn === 'other';

	// The form starts from the category; it's a copy, edited freely.
	// svelte-ignore state_referenced_locally
	const start = category;
	const startLimit = limitToField(start?.speedLimit ?? 0);
	let name = $state(start?.name ?? '');
	// svelte-ignore state_referenced_locally
	let folder = $state(start?.folder ?? suggestFolder(generalFolder, ''));
	let extensions = $state(start?.extensions.join(', ') ?? '');
	let speed = $state(startLimit.value);
	let unit: SpeedUnit = $state(startLimit.unit);
	let running = $state(start?.maxConcurrent ? String(start.maxConcurrent) : '');
	/** Once the user types a folder, the name no longer changes it. */
	let folderTouched = $state(start !== null);
	let saving = $state(false);
	let problem: string | null = $state(null);

	function rename(event: Event & { currentTarget: HTMLInputElement }) {
		name = event.currentTarget.value;
		if (!folderTouched) {
			folder = suggestFolder(generalFolder, name);
		}
	}

	async function browse() {
		const chosen = await pickFolder(folder);
		if (chosen) {
			folder = chosen;
			folderTouched = true;
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		problem = await onSave({
			name,
			folder: other ? null : folder.trim() || null,
			extensions: other ? [] : parseExtensions(extensions),
			speedLimit: fieldToLimit(speed, unit) ?? 0,
			maxConcurrent: Math.max(0, Math.floor(Number(running) || 0))
		});
		saving = false;
		if (problem === null) {
			onClose();
		}
	}
</script>

<!-- Opens in the list, in place of the category's row (or at the top for a
     new one). Escape closes it without saving (the fields keep the keys). -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form
	onsubmit={submit}
	onkeydown={(event) => event.key === 'Escape' && onClose()}
	aria-labelledby="category-editor-title"
>
	<h2 id="category-editor-title">
		{category ? m.cat_editing({ name: categoryName(category) }) : m.cat_new()}
	</h2>

	<div class="fields">
		<label>
			<span>{m.cat_name()}</span>
			<!-- svelte-ignore a11y_autofocus -->
			<input value={name} dir="auto" oninput={rename} required autofocus />
		</label>

		{#if !other}
			<label>
				<span>{m.cat_folder()}</span>
				<span class="row">
					<input bind:value={folder} dir="auto" oninput={() => (folderTouched = true)} />
					{#if canPickFolder()}
						<button type="button" onclick={browse}>
							<FolderOpen size={15} aria-hidden="true" />
							{m.cat_browse()}
						</button>
					{/if}
				</span>
			</label>

			<label class="wide">
				<span>{m.cat_extensions()}</span>
				<input bind:value={extensions} dir="auto" spellcheck="false" autocapitalize="off" />
				<small>{m.cat_extensions_hint()}</small>
			</label>
		{/if}

		<div class="pair wide">
			<label>
				<span>{m.cat_speed()}</span>
				<span class="row">
					<input bind:value={speed} inputmode="decimal" />
					<select bind:value={unit} aria-label={m.cat_speed()}>
						{#each SPEED_UNITS as option (option)}
							<option value={option}>{unitLabel(option)}</option>
						{/each}
					</select>
				</span>
				<small>{m.cat_speed_hint()}</small>
			</label>
			<label>
				<span>{m.cat_running()}</span>
				<input bind:value={running} type="number" min="0" step="1" />
				<small>{m.cat_running_hint()}</small>
			</label>
		</div>
	</div>

	{#if problem}
		<p class="problem" role="alert" in:unfold out:unfold>{problem}</p>
	{/if}

	<div class="buttons">
		<button type="button" onclick={onClose}>{m.cat_cancel()}</button>
		<button class="primary" disabled={saving}>{m.cat_save()}</button>
	</div>
</form>

<style>
	form {
		display: grid;
		gap: 0.9rem;
		padding: 1rem 1.25rem 1.1rem;
		background: var(--well);
	}

	/* Two columns when there's room: name and folder, extensions across,
	   then the limits. */
	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
		gap: 0.9rem 1.25rem;
	}

	.wide {
		grid-column: 1 / -1;
	}

	h2 {
		margin: 0;
		font-size: 1rem;
	}

	label {
		display: grid;
		gap: 0.3rem;
	}

	label > span:first-child {
		font-size: 0.85rem;
		font-weight: 600;
	}

	.row {
		display: flex;
		gap: 0.4rem;
	}

	/* Grows from nothing, so the unit list and the button keep their room. */
	.row input {
		flex: 1 1 0;
		width: 0;
		min-width: 0;
	}

	.row button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		white-space: nowrap;
	}

	select {
		flex: none;
		padding: 0 0.4rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--surface);
		color: var(--text);
		font: inherit;
	}

	small {
		color: var(--muted);
		font-size: 0.8rem;
	}

	.pair {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.9rem;
	}

	.problem {
		margin: 0;
		color: var(--danger);
	}

	.buttons {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	@media (max-width: 30rem) {
		.pair {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
