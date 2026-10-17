<script lang="ts">
	import { onMount } from 'svelte';
	import Popup from '#lib/components/Popup.svelte';
	import { isExtensionOrigin, tokenIn, type ToExtension } from '#lib/popup.js';
	import { loadToken } from '#lib/token.js';

	// The browser extension's popup (docs/decisions/0055), in a frame of the
	// extension's page, outside the shell. The extension hands the token
	// over once this page says it's ready; only a page of an extension (the
	// one around this page) can, and only it hears back. Opened on its own,
	// it uses this browser's sign-in, if any.
	const framed = window.parent !== window;
	let token: string | null = $state(framed ? null : loadToken());
	let extension: string | null = null;

	function tell(message: ToExtension) {
		if (framed) {
			// Nothing secret: whether it's ready, its height, a refusal.
			window.parent.postMessage(message, extension ?? '*');
		}
	}

	onMount(() => {
		const listen = (event: MessageEvent) => {
			const given = tokenIn(event.data);
			if (event.source === window.parent && isExtensionOrigin(event.origin) && given) {
				extension = event.origin;
				token = given;
			}
		};
		addEventListener('message', listen);
		tell({ type: 'ottie-popup-ready' });
		return () => removeEventListener('message', listen);
	});
</script>

{#if token}
	<!-- Keyed: a new token means a new session. -->
	{#key token}
		<Popup {token} {tell} />
	{/key}
{/if}
