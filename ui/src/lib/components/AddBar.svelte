<script lang="ts">
	import type { AddJob } from '@ottie/api-types/AddJob';
	import type { Category } from '@ottie/api-types/Category';
	import type { Existing } from '@ottie/api-types/Existing';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Link from '@lucide/svelte/icons/link';
	import ListPlus from '@lucide/svelte/icons/list-plus';
	import Plus from '@lucide/svelte/icons/plus';
	import {
		automaticLabel,
		defaultFolder,
		existingByDefault,
		jobRequest,
		streamsByDefault,
		type Choice,
		type Looked
	} from '#lib/adding.js';
	import { holdsSeveral } from '#lib/batch.js';
	import { categoryName, type Categories } from '#lib/categories.js';
	import { drop, unfold } from '#lib/moving.svelte.js';
	import { m } from '#lib/paraglide/messages.js';
	import { fromField, inOrder } from '#lib/schedule.js';
	import { canPickFolder, pickFolder } from '#lib/window.js';
	import Switch from './Switch.svelte';

	interface Props {
		categories: Categories;
		/** The same, General first then by name. */
		categoryList: Category[];
		/** The settings start videos and music in streaming mode. */
		streamMedia: boolean;
		/** What the settings do with a file already there, once known. */
		existing: Existing | undefined;
		/** What adding the link would do (in that folder, if given); null if
		 * the daemon can't tell. */
		onLook: (url: string, folder?: string) => Promise<Looked | null>;
		/** Resolves to true once the daemon has accepted the download. */
		onAdd: (job: AddJob) => Promise<boolean>;
		/** Opens the list for adding several links, starting from `text`. */
		onSeveral: (text: string) => void;
	}

	interface Step {
		/** What the daemon said, once it has answered. */
		looked: Looked | null;
	}

	let { categories, categoryList, streamMedia, existing, onLook, onAdd, onSeveral }: Props =
		$props();
	let form: HTMLFormElement;
	let link: HTMLInputElement;
	let url = $state('');
	/** The step after the link (category and folder); null while closed. */
	let step = $state<Step | null>(null);
	let choice = $state<Choice>('auto');
	let folder = $state('');
	/** Streaming mode as the user set it; null leaves it to the daemon. */
	let streaming = $state<boolean | null>(null);
	/** The start and stop time fields (`datetime-local`); empty: none. */
	let start = $state('');
	let stop = $state('');
	/** For a file already there: renamed, or replaced once complete. */
	let onDisk = $state<'rename' | 'replace'>('rename');
	let adding = $state(false);

	let matched = $derived(step?.looked?.matched ?? null);
	/** The name of the file already there, if one is. */
	let taken = $derived(step?.looked?.exists ? step.looked.name : null);
	let fallback = $derived(step && defaultFolder(choice, matched, categories));
	let streams = $derived(streaming ?? streamsByDefault(matched, streamMedia));
	let times = $derived({ start: fromField(start), stop: fromField(stop) });
	let ordered = $derived(inOrder(times.start, times.stop));
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
			// A sequence gives several links: the list shows them.
			if (holdsSeveral(url)) {
				several();
			} else {
				await open();
			}
			return;
		}
		if (!ordered) {
			return;
		}
		adding = true;
		const asked = taken === null ? null : onDisk;
		const added = await onAdd(jobRequest(url, choice, folder, streaming, times, asked));
		adding = false;
		if (added) {
			url = '';
			step = null;
		}
	}

	/** Shows the step, then fills in what the daemon said. */
	async function open() {
		step = { looked: null };
		choice = 'auto';
		folder = '';
		streaming = null;
		start = '';
		stop = '';
		onDisk = existingByDefault(existing);
		await rematch();
	}

	/** Asks the daemon where the link goes now (the folder may decide, and
	 * a file may be there already). */
	async function rematch() {
		const current = step;
		const asked = folder.trim();
		const looked = await onLook(url.trim(), asked || undefined);
		if (step === current && step && folder.trim() === asked) {
			step.looked = looked;
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

	/** Several links pasted at once go to the list instead (the field would
	 * run them together on one line). */
	function pasted(event: ClipboardEvent) {
		const text = event.clipboardData?.getData('text') ?? '';
		if (holdsSeveral(text)) {
			event.preventDefault();
			step = null;
			onSeveral(text);
		}
	}

	/** The list, starting from what's in the field. */
	function several() {
		const text = url;
		url = '';
		step = null;
		onSeveral(text);
	}

	/** A shortcut's request (docs/decisions/0046): `text` in the field with
	 * the step open, or (null) only the cursor in the field. */
	export function take(text: string | null) {
		if (text === null) {
			step = null;
			link.focus();
		} else {
			url = text;
			void open();
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
	<!-- The link and its buttons: one pill. -->
	<div class="bar">
		<label class="field">
			<Link size={15} aria-hidden="true" />
			<input
				bind:this={link}
				bind:value={url}
				dir="auto"
				oninput={() => (step = null)}
				onpaste={pasted}
				type="url"
				placeholder={m.add_placeholder()}
				aria-label={m.add_label()}
				required
			/>
		</label>
		<button
			type="button"
			class="several"
			onclick={several}
			title={m.add_several()}
			aria-label={m.add_several()}
		>
			<ListPlus size={16} aria-hidden="true" />
		</button>
		<button class="primary" disabled={adding || !url.trim()} aria-label={m.add_button()}>
			<Plus size={16} aria-hidden="true" />
			<span>{m.add_button()}</span>
		</button>
	</div>

	{#if step}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="step" onkeydown={keydown} in:drop out:drop>
			<fieldset>
				<legend>{m.add_category()}</legend>
				<div class="choices">
					<label class="choice">
						<!-- svelte-ignore a11y_autofocus -->
						<input type="radio" name="category" value="auto" bind:group={choice} autofocus />
						<span>{automaticLabel(matched, categories)}</span>
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
						dir="auto"
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

			{#if taken !== null}
				<fieldset in:unfold out:unfold>
					<legend>{m.add_exists({ name: taken })}</legend>
					<div class="choices">
						<label class="choice">
							<input type="radio" name="existing" value="rename" bind:group={onDisk} />
							<span>{m.existing_rename()}</span>
						</label>
						<label class="choice">
							<input type="radio" name="existing" value="replace" bind:group={onDisk} />
							<span>{m.existing_replace()}</span>
						</label>
					</div>
					<small>{m.add_exists_hint()}</small>
				</fieldset>
			{/if}

			<div class="streaming">
				<Switch on={streams} label={m.add_streaming()} onChange={(on) => (streaming = on)} />
				<span class="names">
					<span class="legend">{m.add_streaming()}</span>
					<small>{m.add_streaming_hint()}</small>
				</span>
			</div>

			<fieldset>
				<legend>{m.add_when()}</legend>
				<div class="times">
					<label class="time">
						<span class="sub">{m.add_start_at()}</span>
						<input type="datetime-local" bind:value={start} />
					</label>
					<label class="time">
						<span class="sub">{m.add_stop_at()}</span>
						<input type="datetime-local" bind:value={stop} />
					</label>
				</div>
				<small class:problem={!ordered}>
					{ordered ? m.add_when_hint() : m.reason_stop_before_start()}
				</small>
			</fieldset>

			<div class="buttons">
				<button type="button" onclick={close}>{m.add_cancel()}</button>
				<button class="primary" disabled={adding || !ordered}>{m.add_button()}</button>
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

	.bar {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.3rem;
		min-width: 0;
		height: 3rem;
		padding-inline: 1rem 0.3rem;
		border-radius: 999px;
		background: var(--input);
		box-shadow: var(--sunken-shadow);
	}

	.bar:focus-within {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}

	.field {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
		border-radius: var(--radius-s);
		color: var(--muted);
		height: 100%;
		padding: 0;
		border: none;
		background: none;
	}

	.bar button {
		height: 2.4rem;
		border-radius: 999px;
	}

	.several {
		width: 2.4rem;
		justify-content: center;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.several:hover {
		background: var(--hover);
		color: var(--text);
	}

	.bar .primary {
		padding: 0 1.1rem 0 0.9rem;
	}

	.step {
		position: absolute;
		top: calc(100% + 0.4rem);
		inset-inline-end: 0;
		inset-inline-start: 0;
		z-index: 10;
		display: grid;
		gap: 0.9rem;
		padding: 0.9rem;
		border: 1px solid var(--edge);
		border-radius: var(--radius-l);
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

	.times {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
	}

	.time {
		display: grid;
		gap: 0.2rem;
	}

	.sub {
		color: var(--muted);
		font-size: 0.8rem;
	}

	small.problem {
		color: var(--danger);
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
