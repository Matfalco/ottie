<script lang="ts">
	import type { Component } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import Download from '@lucide/svelte/icons/download';
	import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
	import Tags from '@lucide/svelte/icons/tags';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { matches } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';
	import type { Connection } from '#lib/session.svelte.js';
	import LanguagePicker from './LanguagePicker.svelte';

	interface Props {
		connection: Connection;
		jobs: Job[];
	}

	let { connection, jobs }: Props = $props();
	let active = $derived(jobs.filter((job) => matches(job, 'active')).length);

	interface Entry {
		href: string;
		label: () => string;
		icon: Component;
		/** A number beside the name, when there's something to count. */
		badge?: () => number;
	}

	// Settings joins when its page exists.
	const ENTRIES: Entry[] = [
		{ href: resolve('/'), label: m.nav_dashboard, icon: LayoutDashboard },
		{ href: resolve('/downloads'), label: m.nav_downloads, icon: Download, badge: () => active },
		{ href: resolve('/categories'), label: m.nav_categories, icon: Tags }
	];
</script>

<nav aria-label={m.nav_label()} class:offline={connection !== 'live'}>
	<ul>
		{#each ENTRIES as { href, label, icon: Icon, badge } (href)}
			{@const current = page.url.pathname === href}
			{@const count = badge?.() ?? 0}
			<li>
				<a {href} class:current aria-current={current ? 'page' : undefined} title={label()}>
					<Icon size={18} aria-hidden="true" />
					<span class="name">{label()}</span>
					{#if count > 0}
						<span class="badge">{formatCount(count)}</span>
					{/if}
				</a>
			</li>
		{/each}
	</ul>

	<div class="foot">
		<span class="connection {connection}" role="status">
			<span class="dot" aria-hidden="true">●</span>
			<span class="text">
				{#if connection === 'live'}
					{m.connection_live()}
				{:else if connection === 'connecting'}
					{m.connection_connecting()}
				{:else}
					{m.connection_offline()}
				{/if}
			</span>
		</span>
		<div class="language"><LanguagePicker /></div>
	</div>
</nav>

<style>
	nav {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-height: 0;
		padding: 0.75rem 0.6rem;
		background: var(--chrome);
		border-right: 1px solid var(--border);
	}

	ul {
		display: grid;
		gap: 0.15rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	a {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		height: 2.25rem;
		padding: 0 0.65rem;
		border-radius: 8px;
		color: var(--muted);
		text-decoration: none;
	}

	a:hover {
		background: var(--hover);
		color: var(--text);
	}

	a.current {
		background: var(--selected);
		color: var(--text);
		font-weight: 600;
	}

	a.current :global(svg) {
		color: var(--accent);
	}

	.name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.badge {
		min-width: 1.4rem;
		padding: 0 0.4rem;
		border-radius: 999px;
		background: var(--accent);
		color: var(--on-accent);
		font-size: 0.75rem;
		font-weight: 600;
		line-height: 1.25rem;
		text-align: center;
	}

	.foot {
		display: grid;
		gap: 0.6rem;
		margin-top: auto;
		padding: 0 0.4rem;
	}

	.connection {
		color: var(--muted);
		font-size: 0.85em;
	}

	.dot {
		margin-right: 0.35em;
	}

	.live .dot {
		color: var(--progress);
	}

	.offline {
		color: var(--danger);
	}

	/* Medium widths: icons only, like VS Code's activity bar. */
	@media (max-width: 56rem) and (min-width: 40.01rem) {
		nav {
			align-items: center;
			padding: 0.75rem 0.4rem;
		}

		a {
			position: relative;
			justify-content: center;
			width: 2.6rem;
			padding: 0;
		}

		.name,
		.connection .text,
		.language {
			display: none;
		}

		.badge {
			position: absolute;
			top: -0.2rem;
			right: -0.3rem;
			min-width: 1.1rem;
			padding: 0 0.25rem;
			font-size: 0.65rem;
			line-height: 1.1rem;
		}

		.foot {
			padding: 0;
		}
	}

	/* Phones: a bar along the bottom, like an Android app. */
	@media (max-width: 40rem) {
		nav {
			flex-direction: row;
			padding: 0.3rem 0.5rem;
			border-right: none;
			border-top: 1px solid var(--border);
		}

		ul {
			display: flex;
			flex: 1;
			justify-content: space-around;
		}

		li {
			flex: 1;
		}

		a {
			position: relative;
			flex-direction: column;
			justify-content: center;
			gap: 0.1rem;
			height: 3rem;
			font-size: 0.75rem;
		}

		.name {
			flex: none;
		}

		.badge {
			position: absolute;
			top: 0.15rem;
			left: calc(50% + 0.4rem);
			min-width: 1.1rem;
			font-size: 0.65rem;
			line-height: 1.1rem;
		}

		/* Room is short: only a connection problem shows here. The language
		   moves to Settings once that exists. */
		.foot,
		.language {
			display: none;
		}

		nav.offline .foot {
			display: flex;
			align-items: center;
			margin: 0;
		}
	}
</style>
