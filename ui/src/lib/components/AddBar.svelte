<script lang="ts">
	import type { AddJob } from '@ottie/api-types/AddJob';
	import type { Category } from '@ottie/api-types/Category';
	import type { Matched } from '@ottie/api-types/Matched';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Link from '@lucide/svelte/icons/link';
	import Plus from '@lucide/svelte/icons/plus';
	import {
		automaticLabel,
		defaultFolder,
		jobRequest,
		streamsByDefault,
		type Choice
	} from '#lib/adding.js';
	import { categoryName, type Categories } from '#lib/categories.js';
	import { m } from '#lib/paraglide/messages.js';
	import { canPickFolder, pickFolder } from '#lib/window.js';
	import Switch from './Switch.svelte';

	interface Props {
		categories: Categories;
		/** The same, General first then by name. */
		categoryList: Category[];
		/** The settings start videos and music in streaming mode. */
		streamMedia: boolean;
		/** The category the link would get (in that folder, if given); null
		 * if the daemon can't tell. */
		onMatch: (url: string, folder?: string) => Promise<Matched | null>;
		/** Resolves to true once the daemon has accepted the download. */
		onAdd: (job: AddJob) => Promise<boolean>;
	}

	interface Step {
		/** What the daemon matched, once it has answered. */
		matched: Matched | null;
	}

	let { categories, categoryList, streamMedia, onMatch, onAdd }: Props = $props();
	let form: HTMLFormElement;
	let link: HTMLInputElement;
	let url = $state('');
	/** The step after the link (category and folder); null while closed. */
	let step = $state<Step | null>(null);
	let choice = $state<Choice>('auto');
	let folder = $state('');
	/** Streaming mode as the user set it; null leaves it to the daemon. */
	let streaming = $state<boolean | null>(null);
	let adding = $state(false);

	let fallback = $derived(step && defaultFolder(choice, step.matched, categories));
	let streams = $derived(streaming ?? streamsByDefault(step?.matched ?? null, streamMedia));
	/** Other isn't picked: a folder of no category puts a download there. */
	let choices = $derived(categoryList.filter((category) => category.builtIn !== 'other'));
	let otherName = $derived.by(() => {
		const other = categoryList.find((category) => category.builtIn === 'other');
		return other ? categoryName(other) : '';
	});
	let typing: ReturnType<typeof setTimeout> | undefined;

	$effect(() => () => clearTimeout(typing));

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!step) {
			await open();
			return;
		}
		adding = true;
		const added = await onAdd(jobRequest(url, choice, folder, streaming));
		adding = false;
		if (added) {
			url = '';
			step = null;
		}
	}

	/** Shows the step, then fills in what the daemon matched. */
	async function open() {
		step = { matched: null };
		choice = 'auto';
		folder = '';
		streaming = null;
		await rematch();
	}

	/** Asks the daemon where the link goes now (the folder may decide). */
	async function rematch() {
		const current = step;
		const asked = folder.trim();
		const matched = await onMatch(url.trim(), asked || undefined);
		if (step === current && step && folder.trim() === asked) {
			step.matched = matched;
		}
	}

	/** Asks again once the user stops typing for a moment. */
	function folderTyped() {
		clearTimeout(typing);
		typing = setTimeout(() => void rematch(), 300);
	}

	function close() {
		step = null;
		link.focus();
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (
			event.key === 'Enter' &&
			event.target instanceof HTMLInputElement &&
			event.target.type === 'radio'
		) {
			event.preventDefault();
			form.requestSubmit();
		}
	}

	/** A click elsewhere on the page closes the step; the link stays. */
	function pointerdown(event: PointerEvent) {
		if (step && !(event.target instanceof Node && form.contains(event.target))) {
			step = null;
		}
	}

	async function browse() {
		const chosen = await pickFolder(folder || fallback || undefined);
		if (chosen) {
			folder = chosen;
			await rematch();
		}
	}
</script>

<svelte:window onpointerdown={pointerdown} />

<form bind:this={form} onsubmit={submit}>
	<label class="field">
		<Link size={15} aria-hidden="true" />
		<input
			bind:this={link}
			bind:value={url}
			oninput={() => (step = null)}
			type="url"
			placeholder={m.add_placeholder()}
			aria-label={m.add_label()}
			required
		/>
	</label>
	<button class="primary" disabled={adding || !url.trim()} aria-label={m.add_button()}>
		<Plus size={16} aria-hidden="true" />
		<span>{m.add_button()}</span>
	</button>

	{#if step}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="step" onkeydown={keydown}>
			<fieldset>
				<legend>{m.add_category()}</legend>
				<div class="choices">
					<label class="choice">
						<!-- svelte-ignore a11y_autofocus -->
						<input type="radio" name="category" value="auto" bind:group={choice} autofocus />
						<span>{automaticLabel(step.matched, categories)}</span>
					</label>
					{#each choices as category (category.id)}
						<label class="choice">
							<input type="radio" name="category" value={category.id} bind:group={choice} />
							<span>{categoryName(category)}</span>
						</label>
					{/each}
				</div>
				{#if choice === 'auto'}
					<small>
						{folder.trim() ? m.add_folder_hint({ other: otherName }) : m.add_automatic_hint()}
					</small>
				{/if}
			</fieldset>

			<label class="folder">
				<span class="legend">{m.add_folder_label()}</span>
				<span class="row">
					<input
						bind:value={folder}
						oninput={folderTyped}
						placeholder={fallback
							? m.add_folder_default({ folder: fallback })
							: m.add_folder_required()}
					/>
					{#if canPickFolder()}
						<button type="button" onclick={browse}>
							<FolderOpen size={15} aria-hidden="true" />
							{m.cat_browse()}
						</button>
					{/if}
				</span>
			</label>

			<div class="streaming">
				<Switch on={streams} label={m.add_streaming()} onChange={(on) => (streaming = on)} />
				<span class="names">
					<span class="legend">{m.add_streaming()}</span>
					<small>{m.add_streaming_hint()}</small>
				</span>
			</div>

			<div class="buttons">
				<button type="button" onclick={close}>{m.add_cancel()}</button>
				<button class="primary" disabled={adding}>{m.add_button()}</button>
			</div>
		</div>
	{/if}
</form>

<style>
	form {
		position: relative;
		display: flex;
		gap: 0.4rem;
		width: 100%;
	}

	.field {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
		height: 2.1rem;
		padding: 0 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--input);
		color: var(--muted);
	}

	.field:focus-within {
		border-color: var(--accent);
	}

	.field input {
		flex: 1;
		min-width: 0;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		outline: none;
	}

	button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		height: 2.1rem;
		white-space: nowrap;
	}

	.step {
		position: absolute;
		top: calc(100% + 0.4rem);
		right: 0;
		left: 0;
		z-index: 10;
		display: grid;
		gap: 0.9rem;
		padding: 0.9rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		background: var(--surface);
		box-shadow: var(--shadow);
		user-select: text;
	}

	fieldset {
		display: grid;
		gap: 0.45rem;
		margin: 0;
		padding: 0;
		border: none;
	}

	legend,
	.legend {
		margin-bottom: 0.45rem;
		padding: 0;
		font-size: 0.85rem;
		font-weight: 600;
	}

	.legend {
		margin-bottom: 0;
	}

	.choices {
		display: flex;
		flex-wrap: wrap;
		gap: 0.35rem;
	}

	/* A radio button drawn as a chip; the real one stays for the keyboard
	   (arrows move between them) and screen readers. */
	.choice {
		position: relative;
	}

	.choice input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}

	.choice span {
		display: block;
		padding: 0.3rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		cursor: pointer;
	}

	.choice span:hover {
		background: var(--hover);
	}

	.choice input:checked + span {
		border-color: var(--accent);
		background: var(--selected);
	}

	.choice input:focus-visible + span {
		outline: 2px solid var(--focus);
		outline-offset: 1px;
	}

	small {
		color: var(--muted);
		font-size: 0.8rem;
	}

	.folder {
		display: grid;
		gap: 0.35rem;
	}

	.row {
		display: flex;
		gap: 0.4rem;
	}

	.row input {
		flex: 1 1 0;
		width: 0;
		min-width: 0;
	}

	.streaming {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	.names {
		display: grid;
		gap: 0.1rem;
	}

	.buttons {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	/* Narrow: the button keeps only its icon. */
	@media (max-width: 30rem) {
		.primary span {
			display: none;
		}
	}
</style>
