<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, isLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';

	/** Each language in its own words. */
	const NAMES: Record<Locale, string> = { en: 'English', fr: 'Français' };

	function change(event: Event & { currentTarget: HTMLSelectElement }) {
		const chosen = event.currentTarget.value;
		if (isLocale(chosen)) {
			// Remembered by this browser; the page reloads in that language.
			setLocale(chosen);
		}
	}
</script>

<select value={getLocale()} onchange={change} aria-label={m.language()}>
	{#each locales as locale (locale)}
		<option value={locale} lang={locale}>{NAMES[locale]}</option>
	{/each}
</select>

<style>
	select {
		padding: 0.25rem 0.4rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
		color: var(--muted);
		font: inherit;
		font-size: 0.85em;
	}
</style>
