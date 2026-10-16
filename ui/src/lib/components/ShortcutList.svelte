<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { isApple, keyName, shortcutList } from '#lib/shortcuts.js';
	import { canOpenFiles } from '#lib/window.js';

	const apple = isApple(navigator.platform);
	const groups = shortcutList(canOpenFiles());
</script>

<!-- The keyboard shortcuts (docs/decisions/0046), in the "?" dialog and in
     Settings: one grid for every group, so their columns line up. -->
<div class="list">
	{#each groups as group (group.title)}
		<h3>{group.title()}</h3>
		<dl>
			{#each group.entries as entry (entry.label)}
				<div class="row">
					<dt>
						{#each entry.with as held (held)}
							<kbd>{keyName(held, apple)}</kbd>
							<span class="between" aria-hidden="true">+</span>
						{/each}
						{#each entry.keys as key, i (key)}
							{#if i > 0}
								<span class="between" aria-hidden="true">{entry.range ? '–' : '/'}</span>
							{/if}
							<kbd>{keyName(key, apple)}</kbd>
						{/each}
					</dt>
					<dd>{entry.label()}</dd>
				</div>
			{/each}
		</dl>
	{/each}
	<p class="hint">{m.shortcuts_typing()}</p>
</div>

<style>
	.list {
		display: grid;
		grid-template-columns: fit-content(50%) minmax(0, 1fr);
		gap: 0.35rem 1rem;
		align-items: center;
	}

	h3 {
		grid-column: 1 / -1;
		margin: 0.6rem 0 0.05rem;
		color: var(--muted);
		font-size: 0.8rem;
		font-weight: 600;
	}

	h3:first-child {
		margin-top: 0;
	}

	dl,
	.row {
		display: contents;
	}

	dt {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem;
	}

	.between {
		color: var(--faint);
		font-size: 0.8rem;
	}

	kbd {
		min-width: 1.6rem;
		padding: 0.1rem 0.4rem;
		border: 1px solid var(--border);
		border-bottom-width: 2px;
		color: var(--text);
		font-family: var(--mono);
		font-size: 0.78rem;
		text-align: center;
		white-space: nowrap;
		border-color: transparent;
		border-radius: 0.5rem;
		background: var(--raised);
		box-shadow: var(--raised-shadow);
	}

	dd {
		margin: 0;
	}

	.hint {
		grid-column: 1 / -1;
		margin: 0.5rem 0 0;
		color: var(--muted);
		font-size: 0.85rem;
	}
</style>
