// A playlist file (.m3u) holding one play link (docs/decisions/0031). A web
// page can't start VLC, but it can save this file, and the system opens it
// with its media player.

/** The playlist's text. */
export function playlist(link: string): string {
	return `#EXTM3U\n${link}\n`;
}

/** Its file name: the download's, with `.m3u` for an extension. */
export function playlistName(fileName: string): string {
	const dot = fileName.lastIndexOf('.');
	const base = dot > 0 ? fileName.slice(0, dot) : fileName;
	return `${base.trim() || 'ottie'}.m3u`;
}

/** Saves the playlist through the browser's downloads. */
export function savePlaylist(fileName: string, link: string): void {
	const blob = new Blob([playlist(link)], { type: 'audio/x-mpegurl' });
	const url = URL.createObjectURL(blob);
	const save = document.createElement('a');
	save.href = url;
	save.download = playlistName(fileName);
	save.click();
	// Once the browser has taken it.
	setTimeout(() => URL.revokeObjectURL(url), 10_000);
}
