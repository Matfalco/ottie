<script lang="ts">
	// Settings → Connection: Ottie on other computers (docs/decisions/0053).
	// In the desktop app, the list it keeps (tokens in the system's
	// keychain) and shows in its window; in a browser, this address's list,
	// going to the other page.
	import X from '@lucide/svelte/icons/x';
	import { onMount } from 'svelte';
	import { useApp } from '#lib/context.js';
	import { leave, shuffle } from '#lib/moving.svelte.js';
	import { MAX_NAME } from '#lib/others.js';
	import { LiveOthers } from '#lib/others.svelte.js';
	import { m } from '#lib/paraglide/messages.js';

	const { ask } = useApp();
	const others = new LiveOthers();
	onMount(() => others.start());

	let name = $state('');
	let link = $state('');
	let problem: string | null = $state(null);
	let list: HTMLUListElement | undefined = $state();

	// Rows slide into place when one goes (docs/decisions/0048).
	shuffle(
		() => list,
		() => others.entries.map((entry) => String(entry.id))
	);

	async function add(event: SubmitEvent) {
		event.preventDefault();
		problem = await others.add(name, link);
		if (problem === null) {
			name = '';
			link = '';
		}
	}

	async function remove(id: number, shown: string) {
		const message = m.others_remove_confirm({ name: shown });
		if (await ask(message, m.others_remove_yes())) {
			problem = await others.remove(id);
		}
	}

	async function open(id: number | null) {
		problem = await others.open(id);
	}
</script>

<div class="block">
	<span class="names">
		<span>{m.others_title()}</span>
		<span class="hint">{others.app ? m.others_hint_app() : m.others_hint_browser()}</span>
	</span>
	<ul class="rows" bind:this={list}>
		{#each others.entries as entry (entry.id ?? 'here')}
			<li data-flip={String(entry.id)} out:leave>
				<span class="names">
					<span dir="auto">{entry.name}</span>
					<span class="hint address" dir="auto">{entry.address}</span>
				</span>
				{#if entry.current}
					<span class="tag">{m.others_shown()}</span>
				{:else}
					<button disabled={others.busy} onclick={() => void open(entry.id)}>
						{m.others_open()}
					</button>
				{/if}
				{#if entry.id !== null}
					{@const id = entry.id}
					<button
						class="round"
						disabled={others.busy}
						onclick={() => void remove(id, entry.name)}
						aria-label={m.others_remove_label({ name: entry.name })}
						title={m.others_remove_label({ name: entry.name })}
					>
						<X size={15} aria-hidden="true" />
					</button>
				{/if}
			</li>
		{/each}
	</ul>
	<form class="add" onsubmit={add}>
		<input
			class="name"
			bind:value={name}
			maxlength={MAX_NAME}
			dir="auto"
			placeholder={m.others_name()}
			aria-label={m.others_name()}
		/>
		<input
			class="link"
			bind:value={link}
			type="url"
			dir="ltr"
			autocomplete="off"
			placeholder={others.app ? m.others_link() : m.others_link_browser()}
			aria-label={others.app ? m.others_link() : m.others_link_browser()}
		/>
		<button type="submit" disabled={others.busy || link.trim() === ''}>
			{others.busy ? m.others_checking() : m.others_add()}
		</button>
	</form>
	{#if problem}
		<p class="problem" role="alert">{problem}</p>
	{/if}
</div>

<style>
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 0.8rem 1.25rem;
		border-top: 1px solid var(--border-soft);
	}

	.names {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}

	.hint {
		color: var(--muted);
		font-size: 0.85rem;
	}

	.rows {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.rows li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.45rem 0.5rem 0.45rem 0.85rem;
		border-radius: var(--radius-m);
		background: var(--sunken);
	}

	.rows .names {
		flex: 1;
	}

	.rows .names span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.address {
		font-family: var(--mono);
		font-size: 0.8rem;
	}

	.tag {
		flex: none;
		padding: 0.1rem 0.6rem;
		border-radius: var(--radius-pill);
		background: var(--selected);
		color: var(--accent);
		font-size: 0.8rem;
		font-weight: 500;
	}

	.rows button:not(.round) {
		flex: none;
	}

	.round {
		display: grid;
		flex: none;
		place-items: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-radius: var(--radius-pill);
	}

	.add {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}

	.add .name {
		flex: 1 1 8rem;
		min-width: 0;
	}

	.add .link {
		flex: 3 1 14rem;
		min-width: 0;
		font-family: var(--mono);
		font-size: 0.85rem;
	}

	.problem {
		margin: 0;
		color: var(--danger);
	}
</style>
