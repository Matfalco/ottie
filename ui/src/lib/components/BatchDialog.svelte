<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import FileUp from '@lucide/svelte/icons/file-up';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import { automaticLabel, existingByDefault, type Choice } from '#lib/adding.js';
	import {
		SHOWN_AT_ONCE,
		afterAdding,
		choiceOf,
		chooseForAll,
		chosenRows,
		onDiskForAll,
		onDiskRows,
		requests,
		rowNote,
		rowsOf,
		sentRows,
		textOf,
		tickAll,
		type OnDisk,
		type Row
	} from '#lib/batch.js';
	import { categoryName } from '#lib/categories.js';
	import { useApp } from '#lib/context.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import { pop, unfold } from '#lib/moving.svelte.js';
	import { canPickFolder, pickFolder } from '#lib/window.js';
	import FileIcon from './FileIcon.svelte';

	interface Props {
		/** The text to start from: what was pasted, or ''. */
		text: string;
		onClose: () => void;
	}

	let { text: pasted, onClose }: Props = $props();
	const { session, addMany } = useApp();

	let dialog: HTMLDialogElement;
	let picker: HTMLInputElement;
	let text = $state(untrack(() => pasted));
	let folder = $state('');
	let rows: Row[] = $state([]);
	/** The category last chosen for all; new links get it too. */
	let forAll = $state<Choice>('auto');
	/** What the files already there get, last chosen for all of them. */
	let onDisk = $state<OnDisk>(untrack(() => existingByDefault(session.settings?.existing)));
	let looking = $state(false);
	let adding = $state(false);
	/** Why the links couldn't be read or added. */
	let problem: string | null = $state(null);
	/** Some were added, the others not. */
	let notice: string | null = $state(null);
	let shown = $state(SHOWN_AT_ONCE);
	let typing: ReturnType<typeof setTimeout> | undefined;
	/** Answers to an older question are dropped. */
	let asked = 0;

	/** Other isn't picked: a folder of no category puts a download there. */
	let choices = $derived(session.categoryList.filter((category) => category.builtIn !== 'other'));
	let chosen = $derived(chosenRows(rows));
	/** Ticked, less the files already there that are skipped. */
	let sent = $derived(sentRows(rows));
	let onDiskCount = $derived(onDiskRows(rows).length);
	let addable = $derived(rows.filter((row) => row.found.kind === 'link').length);
	let all = $derived(addable > 0 && chosen.length === addable);

	onMount(() => {
		dialog.showModal();
		void look();
		return () => clearTimeout(typing);
	});

	/** Asks the daemon which links the text holds (in that folder). */
	async function look() {
		clearTimeout(typing);
		const question = ++asked;
		if (text.trim() === '') {
			rows = [];
			problem = null;
			return;
		}
		looking = true;
		const answer = await session.preview(text, folder.trim() || undefined);
		if (question !== asked) {
			return;
		}
		looking = false;
		if (answer.ok) {
			rows = rowsOf(answer.value.links, rows, forAll, onDisk);
			problem = null;
		} else {
			problem = answer.why;
		}
	}

	/** Asks again once the user stops typing for a moment. */
	function typed() {
		notice = null;
		clearTimeout(typing);
		typing = setTimeout(() => void look(), 300);
	}

	/** The file's text goes after what's there. */
	async function importFile() {
		const file = picker.files?.[0];
		picker.value = '';
		if (!file) {
			return;
		}
		try {
			const content = await file.text();
			text = text.trim() === '' ? content : `${text.trimEnd()}\n${content}`;
		} catch {
			problem = m.batch_import_failed();
			return;
		}
		await look();
	}

	async function browse() {
		const picked = await pickFolder(folder || undefined);
		if (picked) {
			folder = picked;
			await look();
		}
	}

	function setForAll(value: string) {
		forAll = choiceOf(value);
		rows = chooseForAll(rows, forAll);
	}

	function setOnDisk(value: OnDisk) {
		onDisk = value;
		rows = onDiskForAll(rows, onDisk);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (sent.length === 0) {
			return;
		}
		adding = true;
		const answer = await addMany(requests(sent, folder));
		adding = false;
		if (!answer.ok) {
			problem = answer.why;
			return;
		}
		if (answer.value.refused.length === 0) {
			onClose();
			return;
		}
		const added = answer.value.ids.filter((id) => id !== null).length;
		rows = afterAdding(rows, sent, answer.value);
		text = textOf(rows);
		problem = null;
		notice = m.batch_some_refused({ count: formatCount(added) });
	}
</script>

<dialog
	bind:this={dialog}
	in:pop
	out:pop
	aria-labelledby="batch-title"
	oncancel={(event) => {
		// Escape: the page decides when it closes.
		event.preventDefault();
		onClose();
	}}
>
	<form onsubmit={submit}>
		<h2 id="batch-title">{m.batch_title()}</h2>

		<label class="text">
			<span class="legend">{m.batch_text()}</span>
			<!-- svelte-ignore a11y_autofocus -->
			<textarea
				bind:value={text}
				dir="auto"
				oninput={typed}
				placeholder={m.batch_text_placeholder()}
				rows="4"
				spellcheck="false"
				autofocus></textarea>
		</label>
		<div class="under">
			<small>{m.batch_text_hint()}</small>
			<button type="button" onclick={() => picker.click()}>
				<FileUp size={15} aria-hidden="true" />
				{m.batch_import()}
			</button>
			<input
				bind:this={picker}
				type="file"
				accept=".txt,.csv,.html,.htm,.md,text/plain"
				hidden
				onchange={importFile}
			/>
		</div>

		<div class="list">
			<div class="head">
				<input
					type="checkbox"
					checked={all}
					indeterminate={chosen.length > 0 && !all}
					disabled={addable === 0}
					aria-label={m.batch_tick_all()}
					onchange={() => (rows = tickAll(rows, !all))}
				/>
				<span>{m.batch_found({ count: formatCount(rows.length) })}</span>
				<span class="muted">{m.batch_chosen({ count: formatCount(sent.length) })}</span>
				{#if looking}
					<span class="muted looking">{m.batch_looking()}</span>
				{/if}
			</div>
			<ul>
				{#each rows.slice(0, shown) as row (row.found.kind === 'link' ? row.found.url : row.found.text)}
					{@const note = rowNote(row)}
					{@const skipped = row.found.kind === 'link' && row.found.exists && row.onDisk === 'skip'}
					<li class:off={!row.chosen || skipped}>
						{#if row.found.kind === 'link'}
							{@const found = row.found}
							<input type="checkbox" bind:checked={row.chosen} aria-label={found.name} />
							<FileIcon name={found.name} />
							<span class="names">
								<span class="name" dir="auto">{found.name}</span>
								<span class="url" title={found.url} dir="auto">{found.url}</span>
								{#if note}
									<span class="note" class:bad={row.refused !== null}>{note}</span>
								{/if}
								{#if found.exists}
									<span class="note disk">
										{m.batch_exists()}
										<select
											value={row.onDisk}
											aria-label={m.batch_existing_of({ name: found.name })}
											onchange={(event) => (row.onDisk = event.currentTarget.value as OnDisk)}
										>
											{@render onDiskOptions()}
										</select>
									</span>
								{/if}
							</span>
							<select
								value={String(row.choice)}
								aria-label={m.batch_category_of({ name: found.name })}
								onchange={(event) => (row.choice = choiceOf(event.currentTarget.value))}
							>
								<option value="auto">{automaticLabel(found.matched, session.categories)}</option>
								{#each choices as category (category.id)}
									<option value={String(category.id)}>{categoryName(category)}</option>
								{/each}
							</select>
						{:else}
							<input type="checkbox" disabled aria-label={row.found.text} />
							<FileIcon name="" />
							<span class="names">
								<span class="url" title={row.found.text} dir="auto">{row.found.text}</span>
								<span class="note bad">{note}</span>
							</span>
						{/if}
					</li>
				{:else}
					{#if !looking}
						<li class="empty">{m.batch_none()}</li>
					{/if}
				{/each}
			</ul>
			{#if rows.length > shown}
				<button type="button" class="more" onclick={() => (shown += SHOWN_AT_ONCE)}>
					{m.batch_more({
						count: formatCount(Math.min(SHOWN_AT_ONCE, rows.length - shown))
					})}
				</button>
			{/if}
		</div>

		<div class="choose">
			<label>
				<span class="legend">{m.batch_category_all()}</span>
				<select value={String(forAll)} onchange={(event) => setForAll(event.currentTarget.value)}>
					<option value="auto">{m.batch_automatic_all()}</option>
					{#each choices as category (category.id)}
						<option value={String(category.id)}>{categoryName(category)}</option>
					{/each}
				</select>
			</label>
			<label class="folder">
				<span class="legend">{m.add_folder_label()}</span>
				<span class="row">
					<input
						bind:value={folder}
						dir="auto"
						oninput={typed}
						placeholder={m.batch_folder_default()}
					/>
					{#if canPickFolder()}
						<button type="button" onclick={browse}>
							<FolderOpen size={15} aria-hidden="true" />
							{m.cat_browse()}
						</button>
					{/if}
				</span>
			</label>
		</div>

		{#if onDiskCount > 0}
			<label class="disk-all" in:unfold out:unfold>
				<span class="legend">{m.batch_existing_all({ count: formatCount(onDiskCount) })}</span>
				<select value={onDisk} onchange={(event) => setOnDisk(event.currentTarget.value as OnDisk)}>
					{@render onDiskOptions()}
				</select>
			</label>
		{/if}

		{#if problem}
			<p class="problem" role="alert" in:unfold out:unfold>{problem}</p>
		{:else if notice}
			<p class="notice" role="status" in:unfold out:unfold>{notice}</p>
		{/if}

		<div class="buttons">
			<button type="button" onclick={onClose}>{m.add_cancel()}</button>
			<button class="primary" disabled={adding || sent.length === 0}>
				{m.batch_add({ count: formatCount(sent.length) })}
			</button>
		</div>
	</form>
</dialog>

{#snippet onDiskOptions()}
	<option value="rename">{m.existing_rename()}</option>
	<option value="replace">{m.existing_replace()}</option>
	<option value="skip">{m.existing_skip()}</option>
{/snippet}

<style>
	dialog {
		width: min(48rem, calc(100vw - 2rem));
		max-height: calc(100vh - 2rem);
		padding: 1.25rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow);
	}

	dialog::backdrop {
		background: var(--backdrop);
	}

	form {
		display: grid;
		gap: 0.75rem;
	}

	h2 {
		margin: 0;
		font-size: 1.1em;
		font-weight: 600;
	}

	label {
		display: grid;
		gap: 0.35rem;
	}

	.legend {
		font-size: 0.85rem;
		font-weight: 600;
	}

	textarea {
		width: 100%;
		min-height: 5.5rem;
		padding: 0.45rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-s);
		background: var(--input);
		color: var(--text);
		font: 0.85rem/1.45 var(--mono);
		resize: vertical;
	}

	.under {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		margin-top: -0.35rem;
	}

	small,
	.muted {
		color: var(--muted);
	}

	small {
		font-size: 0.8rem;
	}

	button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		white-space: nowrap;
	}

	.list {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-m);
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.45rem 0.7rem;
		border-bottom: 1px solid var(--border-soft);
		background: var(--well);
		font-size: 0.85rem;
	}

	.looking {
		margin-inline-start: auto;
	}

	ul {
		max-height: min(40vh, 22rem);
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}

	li {
		display: grid;
		grid-template-columns: auto auto minmax(0, 1fr) minmax(0, 13rem);
		align-items: center;
		gap: 0.6rem;
		padding: 0.4rem 0.7rem;
		border-bottom: 1px solid var(--border-soft);
	}

	li:last-child {
		border-bottom: none;
	}

	li.off .names,
	li.off :global(svg) {
		opacity: 0.6;
	}

	li.empty {
		display: block;
		color: var(--muted);
	}

	.names {
		display: grid;
		min-width: 0;
	}

	.name,
	.url {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.url {
		color: var(--muted);
		font: 0.78rem var(--mono);
	}

	.note {
		color: var(--muted);
		font-size: 0.8rem;
	}

	.note.bad {
		color: var(--danger);
	}

	/* A file already there: what it gets, beside the note. */
	.note.disk {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.note.disk select {
		width: auto;
		padding: 0.1rem 0.4rem;
		font-size: 0.8rem;
	}

	.disk-all {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.35rem 0.75rem;
	}

	li select {
		width: 100%;
		padding: 0.25rem 0.5rem;
		font-size: 0.85rem;
	}

	.more {
		justify-content: center;
		border: none;
		border-top: 1px solid var(--border-soft);
		border-radius: 0;
		background: none;
		color: var(--accent);
	}

	.choose {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: 0.75rem;
	}

	/* No wider than its column, however long its options. */
	.choose select {
		width: 100%;
		min-width: 0;
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

	p {
		margin: 0;
		font-size: 0.85em;
	}

	.problem {
		color: var(--danger);
	}

	.buttons {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	/* Narrow: the category menu under the name, the two fields stacked. */
	@media (max-width: 36rem) {
		li {
			grid-template-columns: auto auto minmax(0, 1fr);
		}

		li select {
			grid-column: 3;
		}

		.choose {
			grid-template-columns: minmax(0, 1fr);
		}

		.under {
			flex-wrap: wrap;
		}
	}
</style>
