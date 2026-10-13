<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { getLocale, isLocale, locales, setLocale, type Locale } from '#lib/paraglide/runtime.js';

	interface Props {
		/** For a `<label for>` outside. */
		id?: string;
	}

	let { id }: Props = $props();

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

<select {id} value={getLocale()} onchange={change} aria-label={id ? undefined : m.language()}>
	{#each locales as locale (locale)}
		<option value={locale} lang={locale}>{NAMES[locale]}</option>
	{/each}
</select>

<style>
	select {
		min-width: 10rem;
		padding: 0.35rem 0.6rem;
	}
</style>
