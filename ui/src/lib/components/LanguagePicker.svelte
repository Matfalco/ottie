<script lang="ts">
	import {
		chosenLanguage,
		NAMES,
		sortedLocales,
		switchLanguage,
		systemLanguage
	} from '#lib/languages.js';
	import { m } from '#lib/paraglide/messages.js';
	import { isLocale, type Locale } from '#lib/paraglide/runtime.js';

	interface Props {
		/** For a `<label for>` outside. */
		id?: string;
		/** Switches to the language chosen (null: the system's); by default,
		 * on this device only. */
		onChoose?: (locale: Locale | null) => void;
	}

	let { id, onChoose = switchLanguage }: Props = $props();

	const SYSTEM = 'system';
	const system = systemLanguage();

	function change(event: Event & { currentTarget: HTMLSelectElement }) {
		const chosen = event.currentTarget.value;
		if (chosen === SYSTEM) {
			onChoose(null);
		} else if (isLocale(chosen)) {
			// Remembered by this browser; the page reloads in that language.
			onChoose(chosen);
		}
	}
</script>

<select
	{id}
	value={chosenLanguage() ?? SYSTEM}
	onchange={change}
	aria-label={id ? undefined : m.language()}
>
	<option value={SYSTEM}>{m.language_system({ language: NAMES[system] })}</option>
	{#each sortedLocales() as locale (locale)}
		<option value={locale} lang={locale}>{NAMES[locale]}</option>
	{/each}
</select>

<style>
	select {
		min-width: 10rem;
		max-width: 100%;
		padding: 0.35rem 0.6rem;
	}

	/* Each name in its own language's letterforms: Chinese, Japanese and
	   Korean draw some of the same characters differently (日本語, 繁體中文). */
	option[lang='zh-Hans'] {
		font-family: 'Ottie Noto Sans SC', var(--font);
	}
	option[lang='zh-Hant'] {
		font-family: 'Ottie Noto Sans TC', var(--font);
	}
	option[lang='ja'] {
		font-family: 'Ottie Noto Sans JP', var(--font);
	}
	option[lang='ko'] {
		font-family: 'Ottie Noto Sans KR', var(--font);
	}
</style>
