<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import Folder from '@lucide/svelte/icons/folder';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash from '@lucide/svelte/icons/trash';
	import { GENERAL, categoryName, describeLimits } from '#lib/categories.js';
	import { useApp } from '#lib/context.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import CategoryEditor from './CategoryEditor.svelte';

	const { session, ask } = useApp();

	/** The category in the editor: `null` for a new one, `undefined` when closed. */
	let editing: Category | null | undefined = $state(undefined);

	let counts = $derived(
		session.list.reduce<Record<number, number>>((counts, job) => {
			counts[job.category] = (counts[job.category] ?? 0) + 1;
			return counts;
		}, {})
	);
	let generalFolder = $derived(session.categories[GENERAL]?.folder ?? null);

	async function remove(category: Category) {
		if (await ask(m.cat_remove_confirm({ name: categoryName(category) }), m.cat_remove())) {
			await session.removeCategory(category.id);
		}
	}
</script>

<div class="page">
	<header>
		<div>
			<h1>{m.cat_title()}</h1>
			<p>{m.cat_intro()}</p>
		</div>
		<button class="primary" onclick={() => (editing = null)}>
			<Plus size={16} aria-hidden="true" />
			{m.cat_new()}
		</button>
	</header>

	<ul>
		{#each session.categoryList as category (category.id)}
			<li class="card">
				<div class="top">
					<h2>{categoryName(category)}</h2>
					{#if category.id === GENERAL}
						<span class="badge">{m.cat_default()}</span>
					{/if}
					<span class="actions">
						<button
							onclick={() => (editing = category)}
							aria-label={m.cat_edit_label({ name: categoryName(category) })}
							title={m.cat_edit()}
						>
							<Pencil size={15} />
						</button>
						{#if category.id !== GENERAL}
							<button
								onclick={() => remove(category)}
								aria-label={m.cat_remove_label({ name: categoryName(category) })}
								title={m.cat_remove()}
							>
								<Trash size={15} />
							</button>
						{/if}
					</span>
				</div>

				<p class="folder" title={category.folder ?? undefined}>
					<Folder size={14} aria-hidden="true" />
					<span>{category.folder ?? m.cat_no_folder()}</span>
				</p>

				{#if category.extensions.length > 0}
					<p class="extensions">
						{#each category.extensions as extension (extension)}
							<span class="chip">{extension}</span>
						{/each}
					</p>
				{:else}
					<p class="muted">{m.cat_no_extensions()}</p>
				{/if}

				<p class="muted">
					{describeLimits(category)} · {m.cat_downloads({
						count: formatCount(counts[category.id] ?? 0)
					})}
				</p>
			</li>
		{/each}
	</ul>
</div>

{#if editing !== undefined}
	<CategoryEditor
		category={editing}
		{generalFolder}
		onSave={(body) => session.saveCategory(editing?.id ?? null, body)}
		onClose={() => (editing = undefined)}
	/>
{/if}

<style>
	.page {
		display: grid;
		align-content: start;
		gap: 1.25rem;
		width: 100%;
		max-width: 72rem;
		padding: 1.25rem;
		overflow: auto;
	}

	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
	}

	h1 {
		margin: 0;
		font-size: 1.35rem;
	}

	header p {
		max-width: 40rem;
		margin: 0.3rem 0 0;
		color: var(--muted);
	}

	header button {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.4rem;
	}

	ul {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
		gap: 1rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.card {
		display: grid;
		align-content: start;
		gap: 0.55rem;
		padding: 1rem 1.1rem;
		border: 1px solid var(--border);
		border-radius: 12px;
		background: var(--surface);
	}

	.top {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2rem;
	}

	h2 {
		margin: 0;
		overflow: hidden;
		font-size: 1.05rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.badge {
		padding: 0.05rem 0.5rem;
		border-radius: 999px;
		background: var(--selected);
		color: var(--text);
		font-size: 0.75rem;
	}

	.actions {
		display: flex;
		gap: 0.15rem;
		margin-left: auto;
	}

	.actions button {
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-color: transparent;
		background: none;
		color: var(--muted);
	}

	.actions button:hover {
		background: var(--hover);
		color: var(--text);
	}

	p {
		margin: 0;
	}

	.folder {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		min-width: 0;
		color: var(--muted);
		font-size: 0.9em;
	}

	.folder :global(svg) {
		flex: none;
	}

	.folder span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.extensions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.chip {
		padding: 0.05rem 0.45rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		color: var(--muted);
		font-size: 0.8rem;
	}

	.muted {
		color: var(--muted);
		font-size: 0.85em;
	}

	@media (max-width: 40rem) {
		.page {
			padding: 0.75rem;
		}

		header {
			flex-direction: column;
		}

		ul {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
