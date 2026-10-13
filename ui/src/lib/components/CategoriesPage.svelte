<script lang="ts">
	import type { Category } from '@ottie/api-types/Category';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash from '@lucide/svelte/icons/trash';
	import {
		GENERAL,
		categoryHue,
		categoryName,
		describeLimits,
		someExtensions
	} from '#lib/categories.js';
	import { useApp } from '#lib/context.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import CategoryEditor from './CategoryEditor.svelte';
	import LinkTester from './LinkTester.svelte';

	const { session, ask } = useApp();
	/** How many extensions a row shows before "+3 more". */
	const SHOWN_EXTENSIONS = 6;

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

{#snippet editor(category: Category | null)}
	<li class="editing">
		<CategoryEditor
			{category}
			{generalFolder}
			onSave={(body) => session.saveCategory(category?.id ?? null, body)}
			onClose={() => (editing = undefined)}
		/>
	</li>
{/snippet}

<div class="page">
	<div class="top">
		<div class="intro">
			<h1>{m.cat_title()}</h1>
			<p>{m.cat_intro()}</p>
		</div>
		<div class="tester">
			<LinkTester categories={session.categories} onMatch={(url) => session.match(url)} />
		</div>
	</div>

	<section class="pane" aria-labelledby="categories-title">
		<div class="head">
			<!-- Only once loaded: "0" while connecting would be wrong. -->
			<h2 id="categories-title">
				{session.connection === 'live'
					? m.cat_count({ count: formatCount(session.categoryList.length) })
					: ''}
			</h2>
			<button class="primary" onclick={() => (editing = null)} disabled={editing === null}>
				<Plus size={16} aria-hidden="true" />
				{m.cat_new()}
			</button>
		</div>

		<ul>
			{#if editing === null}
				{@render editor(null)}
			{/if}
			{#each session.categoryList as category (category.id)}
				{#if editing?.id === category.id}
					{@render editor(category)}
				{:else}
					{@const name = categoryName(category)}
					{@const [extensions, more] = someExtensions(category.extensions, SHOWN_EXTENSIONS)}
					<li class="row">
						<span class="tile" style:--hue={categoryHue(category)} aria-hidden="true">
							{name.charAt(0).toUpperCase()}
						</span>
						<div class="who">
							<span class="name">
								<span>{name}</span>
								{#if category.builtIn === 'general'}
									<span class="badge">{m.cat_default()}</span>
								{:else if category.builtIn === 'other'}
									<span class="badge">{m.cat_built_in()}</span>
								{/if}
							</span>
							{#if category.builtIn !== 'other'}
								<span class="folder" title={category.folder ?? undefined}>
									{category.folder ?? m.cat_no_folder()}
								</span>
							{/if}
						</div>
						<div class="extensions">
							{#if category.builtIn === 'other'}
								<span class="note">{m.cat_other_folder()}</span>
							{:else if category.builtIn === 'general' && extensions.length === 0}
								<span class="note">{m.cat_general_extensions()}</span>
							{:else if extensions.length === 0}
								<span class="note">{m.cat_no_extensions()}</span>
							{:else}
								{#each extensions as extension (extension)}
									<span class="chip">{extension}</span>
								{/each}
								{#if more > 0}
									<span class="note">{m.cat_more({ count: formatCount(more) })}</span>
								{/if}
							{/if}
						</div>
						<span class="limits">{describeLimits(category)}</span>
						<span class="count">
							{m.cat_downloads({ count: formatCount(counts[category.id] ?? 0) })}
						</span>
						<span class="actions">
							<button
								onclick={() => (editing = category)}
								aria-label={m.cat_edit_label({ name })}
								title={m.cat_edit()}
							>
								<Pencil size={15} />
							</button>
							{#if category.builtIn === null}
								<button
									onclick={() => remove(category)}
									aria-label={m.cat_remove_label({ name })}
									title={m.cat_remove()}
								>
									<Trash size={15} />
								</button>
							{/if}
						</span>
					</li>
				{/if}
			{/each}
		</ul>
	</section>
</div>

<style>
	.page {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 0.75rem;
		min-height: 0;
		padding: 0.25rem 1rem 1rem;
		overflow: auto;
	}

	.top {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.intro {
		display: flex;
		flex: 999 1 26rem;
		flex-direction: column;
		justify-content: center;
		gap: 0.3rem;
		padding: 0.25rem;
	}

	.tester {
		flex: 1 1 22rem;
	}

	h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.intro p {
		max-width: 42rem;
		margin: 0;
		color: var(--muted);
	}

	.pane {
		border: 1px solid var(--border);
		border-radius: var(--radius-l);
		background: var(--surface);
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.6rem 0.75rem 0.6rem 1.15rem;
		border-bottom: 1px solid var(--border-soft);
	}

	h2 {
		margin: 0;
		color: var(--muted);
		font-size: 0.95rem;
		font-weight: 400;
	}

	.head button {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-left: auto;
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		border-bottom: 1px solid var(--border-soft);
	}

	li:last-child {
		border-bottom: none;
	}

	/* One line per category on wide panes; the parts wrap as it narrows. */
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1rem;
		padding: 0.75rem 0.75rem 0.75rem 1.15rem;
	}

	.tile {
		display: grid;
		flex: none;
		place-items: center;
		width: 2.2rem;
		height: 2.2rem;
		border-radius: var(--radius-s);
		background: color-mix(in srgb, var(--hue) 18%, var(--surface));
		color: var(--hue);
		font-weight: 700;
	}

	.who {
		display: grid;
		flex: 1 1 14rem;
		min-width: 0;
	}

	.name {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 1rem;
		font-weight: 600;
	}

	.badge {
		padding: 0 0.5rem;
		border-radius: 999px;
		background: var(--well);
		color: var(--muted);
		font-size: 0.72rem;
		font-weight: 400;
		line-height: 1.2rem;
	}

	.folder {
		overflow: hidden;
		color: var(--muted);
		font-family: var(--mono);
		font-size: 0.8rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.extensions {
		display: flex;
		flex: 1 1 16rem;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.3rem;
	}

	.chip {
		padding: 0 0.5rem;
		border: 1px solid var(--border-soft);
		border-radius: 999px;
		background: var(--well);
		font-family: var(--mono);
		font-size: 0.75rem;
		line-height: 1.3rem;
	}

	.note,
	.limits {
		color: var(--faint);
		font-size: 0.85rem;
	}

	.limits {
		flex: 0 1 17rem;
	}

	.count {
		flex: none;
		color: var(--muted);
		text-align: right;
		white-space: nowrap;
	}

	.actions {
		display: flex;
		flex: 0 0 4.2rem;
		justify-content: flex-end;
		gap: 0.15rem;
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

	@media (max-width: 40rem) {
		.page {
			padding: 0.25rem 0.5rem 0.75rem;
		}

		.count {
			flex-basis: auto;
			text-align: left;
		}
	}
</style>
