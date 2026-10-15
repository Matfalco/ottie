<script lang="ts">
	import type { Component } from 'svelte';
	import type { Job } from '@ottie/api-types/Job';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import Download from '@lucide/svelte/icons/download';
	import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
	import Settings from '@lucide/svelte/icons/settings';
	import Tags from '@lucide/svelte/icons/tags';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { isActive } from '#lib/filters.js';
	import { formatCount } from '#lib/format.js';
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		jobs: Job[];
		/** Icons only, their names in tooltips: the title bar is short of room. */
		compact?: boolean;
	}

	let { jobs, compact = false }: Props = $props();
	let active = $derived(jobs.filter(isActive).length);

	interface Entry {
		href: string;
		label: () => string;
		icon: Component;
		/** A number beside the name, when there's something to count. */
		badge?: () => number;
	}

	const ENTRIES: Entry[] = [
		{ href: resolve('/'), label: m.nav_dashboard, icon: LayoutDashboard },
		{ href: resolve('/downloads'), label: m.nav_downloads, icon: Download, badge: () => active },
		{ href: resolve('/categories'), label: m.nav_categories, icon: Tags },
		{ href: resolve('/schedule'), label: m.nav_schedule, icon: CalendarClock },
		{ href: resolve('/settings'), label: m.nav_settings, icon: Settings }
	];
</script>

<!-- The sections as tabs in the title bar; a bar along the bottom on phones. -->
<nav aria-label={m.nav_label()} class:compact>
	<ul>
		{#each ENTRIES as { href, label, icon: Icon, badge } (href)}
			{@const current = page.url.pathname === href}
			{@const count = badge?.() ?? 0}
			<li>
				<a {href} class:current aria-current={current ? 'page' : undefined} title={label()}>
					<span class="icon"><Icon size={18} aria-hidden="true" /></span>
					<span class="name">{label()}</span>
					{#if count > 0}
						<span class="badge">{formatCount(count)}</span>
					{/if}
				</a>
			</li>
		{/each}
	</ul>
</nav>

<style>
	ul {
		display: flex;
		gap: 2px;
		margin: 0;
		padding: 3px;
		border-radius: var(--radius-s);
		background: var(--input);
		list-style: none;
	}

	a {
		position: relative;
		display: flex;
		align-items: center;
		gap: 0.45rem;
		height: 1.9rem;
		padding: 0 0.85rem;
		border-radius: var(--radius-s);
		color: var(--muted);
		text-decoration: none;
		white-space: nowrap;
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

	.icon {
		display: grid;
	}

	.badge {
		min-width: 1.25rem;
		padding: 0 0.35rem;
		border-radius: 999px;
		background: var(--accent);
		color: var(--on-accent);
		font-size: 0.72rem;
		font-weight: 600;
		line-height: 1.15rem;
		text-align: center;
	}

	/* Short of room in the title bar: icons only. */
	@media (min-width: 40.01rem) {
		.compact a {
			padding: 0 0.6rem;
		}

		.compact .name {
			display: none;
		}

		.compact .badge {
			position: absolute;
			top: -0.3rem;
			right: -0.35rem;
			min-width: 1.05rem;
			padding: 0 0.25rem;
			font-size: 0.65rem;
			line-height: 1.05rem;
		}
	}

	/* Phones: a bar along the bottom, like an Android app. */
	@media (max-width: 40rem) {
		nav {
			position: fixed;
			right: 0;
			bottom: 0;
			left: 0;
			z-index: 5;
			padding: 0.3rem 0.5rem;
			border-top: 1px solid var(--border);
			background: var(--surface);
		}

		ul {
			justify-content: space-around;
			padding: 0;
			background: none;
		}

		li {
			flex: 1;
		}

		a {
			flex-direction: column;
			justify-content: center;
			gap: 0.1rem;
			height: 3rem;
			padding: 0;
			font-size: 0.75rem;
		}

		a.current {
			background: none;
			color: var(--accent);
		}

		/* Five names don't fit side by side: only the current tab's. */
		a:not(.current) .name {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
		}

		.badge {
			position: absolute;
			top: 0.15rem;
			left: calc(50% + 0.4rem);
			min-width: 1.1rem;
			font-size: 0.65rem;
			line-height: 1.1rem;
		}
	}
</style>
