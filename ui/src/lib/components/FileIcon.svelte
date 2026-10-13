<script lang="ts">
	import type { Component } from 'svelte';
	import Disc from '@lucide/svelte/icons/disc-3';
	import File from '@lucide/svelte/icons/file';
	import FileArchive from '@lucide/svelte/icons/file-archive';
	import FileImage from '@lucide/svelte/icons/file-image';
	import FileMusic from '@lucide/svelte/icons/file-music';
	import FileText from '@lucide/svelte/icons/file-text';
	import FileVideo from '@lucide/svelte/icons/file-video-camera';
	import { fileKind, type FileKind } from '#lib/filetypes.js';

	interface Props {
		/** The file's name; its extension picks the icon. */
		name: string;
		size?: number;
	}

	let { name, size = 16 }: Props = $props();

	const ICONS: Record<FileKind, Component> = {
		video: FileVideo,
		audio: FileMusic,
		image: FileImage,
		archive: FileArchive,
		document: FileText,
		program: Disc,
		other: File
	};

	let Icon = $derived(ICONS[fileKind(name)]);
</script>

<Icon {size} aria-hidden="true" />
