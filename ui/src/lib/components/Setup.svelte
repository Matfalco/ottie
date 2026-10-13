<script lang="ts">
	import type { CategoryBody } from '@ottie/api-types/CategoryBody';
	import Folder from '@lucide/svelte/icons/folder';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Plus from '@lucide/svelte/icons/plus';
	import icon from '#lib/assets/ottie.svg';
	import { GENERAL } from '#lib/categories.js';
	import { useApp } from '#lib/context.js';
	import { m } from '#lib/paraglide/messages.js';
	import { asCategory, edited, folderOf, starters, toCreate, type Starter } from '#lib/setup.js';
	import { canPickFolder, pickFolder } from '#lib/window.js';
	import CategoryEditor from './CategoryEditor.svelte';

	const { session } = useApp();

	// The settings and the categories arrive together, so General is known.
	let downloads = $state(session.categories[GENERAL]?.folder ?? '');
	let list: Starter[] = $state(starters());
	/** The row in the editor: `null` for a new one, `undefined` when closed. */
	let editing: Starter | null | undefined = $state(undefined);
	let added = 0;
	let saving = $state(false);
	let problem: string | null = $state(null);

	async function browse() {
		const chosen = await pickFolder(downloads);
		if (chosen) {
			downloads = chosen;
		}
	}

	async function save(body: CategoryBody): Promise<string | null> {
		if (editing) {
			const key = editing.key;
			const index = list.findIndex((starter) => starter.key === key);
			list[index] = edited(editing, body, downloads);
		} else {
			added += 1;
			const fresh: Starter = { key: `added-${added}`, body, chosen: true, folderFollows: false };
			list.push(edited(fresh, body, downloads));
		}
		return null;
	}

	async function finish(skip: boolean) {
		saving = true;
		problem = skip
			? await session.finishSetup(null, [])
			: await session.finishSetup(
					downloads.trim() || null,
					toCreate(list, downloads, session.categoryList)
				);
		saving = false;
	}
</script>

<div class="page">
	<div class="content">
		<header>
			<img src={icon} alt="" width="48" height="48" />
			<div>
				<h1>{m.setup_title()}</h1>
				<p>{m.setup_intro()}</p>
			</div>
		</header>

		<label class="downloads">
			<span class="heading">{m.setup_folder()}</span>
			<span class="row">
				<input bind:value={downloads} />
				{#if canPickFolder()}
					<button type="button" onclick={browse}>
						<FolderOpen size={15} aria-hidden="true" />
						{m.cat_browse()}
					</button>
				{/if}
			</span>
			<small>{m.setup_folder_hint()}</small>
		</label>

		<section>
			<h2 class="heading">{m.cat_title()}</h2>
			<ul>
				{#each list as starter (starter.key)}
					{@const folder = folderOf(starter, downloads)}
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
								<span>{folder ?? m.cat_no_folder()}</span>
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
			<button class="add" onclick={() => (editing = null)}>
				<Plus size={16} aria-hidden="true" />
				{m.setup_add()}
			</button>
		</section>

		{#if problem}
			<p class="problem" role="alert">{problem}</p>
		{/if}

		<div class="buttons">
			<button disabled={saving} onclick={() => finish(true)}>{m.setup_skip()}</button>
			<button class="primary" disabled={saving} onclick={() => finish(false)}>
				{m.setup_finish()}
			</button>
		</div>
	</div>
</div>

{#if editing !== undefined}
	<CategoryEditor
		category={editing ? asCategory(editing, downloads) : null}
		generalFolder={downloads.trim() || null}
		onSave={save}
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
		gap: 1.5rem;
		max-width: 44rem;
		margin: 0 auto;
		padding: 2rem 1.25rem;
	}

	header {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
	}

	h1 {
		margin: 0;
		font-size: 1.5rem;
	}

	header p {
		margin: 0.4rem 0 0;
		color: var(--muted);
	}

	.heading {
		margin: 0;
		font-size: 0.85rem;
		font-weight: 600;
	}

	.downloads {
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

	button {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		white-space: nowrap;
	}

	small {
		color: var(--muted);
		font-size: 0.8rem;
	}

	section {
		display: grid;
		gap: 0.5rem;
	}

	ul {
		margin: 0;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 0.75rem;
	}

	li + li {
		border-top: 1px solid var(--border-soft);
	}

	li.off .about {
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

	.add {
		justify-self: start;
	}

	.problem {
		margin: 0;
		color: var(--danger);
	}

	.buttons {
		display: flex;
		justify-content: flex-end;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	@media (max-width: 30rem) {
		.about {
			grid-template-columns: minmax(0, 1fr);
		}

		.name {
			grid-row: auto;
		}
	}
</style>
