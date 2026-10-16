<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Api } from '#lib/api.js';
	import icon from '#lib/assets/ottie.svg';
	import {
		LISTED,
		SHOWN_FOR,
		caughtRows,
		caughtTitle,
		othersText,
		outcome,
		type CaughtRow
	} from '#lib/caught.js';
	import { m } from '#lib/paraglide/messages.js';
	import { loadToken } from '#lib/token.js';
	import { caughtDone, caughtLinks, caughtMore, caughtReady, followCaught } from '#lib/window.js';
	import FileIcon from './FileIcon.svelte';

	// The desktop app's window for links copied elsewhere (docs/decisions/0043):
	// in the screen's corner, sized to what's drawn here, and never taking the
	// focus. The app says which links it caught; the page asks the daemon
	// what they are, and adds them. Left alone, it goes away by itself.

	const token = loadToken();
	const api = token ? new Api(token) : null;

	/** The links offered, as the app has them. */
	let offered: string[] = $state([]);
	let rows: CaughtRow[] = $state([]);
	/** Why the daemon refused some, by link. */
	let refused: Record<string, string> = $state({});
	let failed = $state(false);
	let adding = $state(false);
	const addable = $derived(offered.filter((url) => !(url in refused)));
	const others = $derived(othersText(rows.length));

	let box: HTMLElement;
	let timer: ReturnType<typeof setTimeout> | undefined;
	let pointed = false;
	/** Answers to an older question are dropped. */
	let asked = 0;

	onMount(() => {
		const unfollow = followCaught(() => void load());
		const observer = new ResizeObserver(report);
		observer.observe(box);
		void load();
		return () => {
			unfollow();
			observer.disconnect();
			clearTimeout(timer);
		};
	});

	/** Asks the app for the links, then the daemon what they are. */
	async function load() {
		const question = ++asked;
		try {
			const links = await caughtLinks();
			const [preview, categories] =
				links.length > 0 && api
					? await Promise.all([api.previewLinks(links.join('\n')), api.listCategories()])
					: [{ links: [] }, []];
			if (question !== asked) {
				return;
			}
			offered = links;
			rows = caughtRows(
				preview.links,
				Object.fromEntries(categories.map((category) => [category.id, category]))
			);
			failed = links.length > 0 && !api;
		} catch {
			if (question !== asked) {
				return;
			}
			failed = true;
			rows = [];
		}
		// The app shows the window once it knows how tall it is.
		await tick();
		report();
		restart();
	}

	function report() {
		caughtReady(Math.ceil(box.getBoundingClientRect().height));
	}

	/** Its time starts again: it goes away unless the pointer is on it. */
	function restart() {
		clearTimeout(timer);
		if (!pointed && offered.length > 0) {
			timer = setTimeout(ignore, SHOWN_FOR);
		}
	}

	function point(on: boolean) {
		pointed = on;
		restart();
	}

	function ignore() {
		void caughtDone(offered).catch(() => {});
	}

	function more() {
		void caughtMore(offered).catch(() => {});
	}

	async function download() {
		if (!api) {
			return;
		}
		adding = true;
		const sent = addable;
		try {
			const answer = outcome(sent, await api.addJobs(sent.map((url) => ({ url }))));
			refused = { ...refused, ...answer.refused };
			// Those refused stay, saying why.
			await caughtDone(answer.added);
		} catch {
			failed = true;
		}
		adding = false;
	}
</script>

<div
	class="caught"
	role="region"
	aria-labelledby="caught-title"
	bind:this={box}
	onpointerenter={() => point(true)}
	onpointerleave={() => point(false)}
>
	<header>
		<img src={icon} alt="" width="22" height="22" />
		<h1 id="caught-title">{caughtTitle(Math.max(1, offered.length))}</h1>
	</header>
	{#if failed}
		<p class="problem" role="alert">{m.caught_failed()}</p>
	{:else if rows.length > 0}
		<ul>
			{#each rows.slice(0, LISTED) as row (row.url)}
				<li>
					<FileIcon name={row.name} size={18} />
					<span class="names">
						<span class="name" dir="auto" title={row.url}>{row.name}</span>
						{#if refused[row.url]}
							<span class="refused">{refused[row.url]}</span>
						{:else}
							<span class="about">
								{#if row.category}<span>{row.category}</span>{/if}
								<span dir="auto">{row.host}</span>
							</span>
						{/if}
					</span>
				</li>
			{/each}
		</ul>
		{#if others}
			<p class="others">{others}</p>
		{/if}
	{/if}
	<div class="buttons">
		<button onclick={ignore}>{m.caught_ignore()}</button>
		<button onclick={more} title={m.caught_more_hint()}>{m.caught_more()}</button>
		<button class="primary" disabled={adding || addable.length === 0 || !api} onclick={download}>
			{m.add_button()}
		</button>
	</div>
</div>

<style>
	:global(html),
	:global(body) {
		background: var(--surface);
	}

	.caught {
		display: grid;
		gap: 0.75rem;
		padding: 0.9rem 1rem 1rem;
		border: 1px solid var(--border);
		background: var(--surface);
		color: var(--text);
		user-select: none;
	}

	header {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}

	h1 {
		margin: 0;
		font-size: 0.95rem;
		font-weight: 600;
	}

	ul {
		display: grid;
		gap: 0.5rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-width: 0;
		color: var(--muted);
	}

	li :global(svg) {
		flex: none;
	}

	.names {
		display: grid;
		flex: 1;
		min-width: 0;
	}

	.name,
	.about,
	.refused {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.name {
		color: var(--text);
		font-weight: 500;
		text-align: start;
	}

	.about {
		font-size: 0.8rem;
	}

	.about span + span::before {
		content: '·';
		margin-inline: 0.35em;
	}

	.refused {
		color: var(--danger);
		font-size: 0.8rem;
	}

	p {
		margin: 0;
		font-size: 0.85rem;
	}

	.others {
		color: var(--muted);
	}

	.problem {
		color: var(--danger);
	}

	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 0.5rem;
	}
</style>
