// What kind of file a download is, from its extension: for its icon now, and
// for filtering by type later.

export type FileKind = 'video' | 'audio' | 'image' | 'archive' | 'document' | 'program' | 'other';

/** The kinds with a list of extensions. */
export type KnownKind = Exclude<FileKind, 'other'>;

export const EXTENSIONS: Record<KnownKind, string[]> = {
	video: ['mp4', 'mkv', 'webm', 'avi', 'mov', 'm4v', 'wmv', 'flv', 'mpg', 'mpeg', 'ts', '3gp'],
	audio: ['mp3', 'flac', 'ogg', 'opus', 'm4a', 'aac', 'wav', 'wma', 'aiff'],
	image: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'tif', 'tiff', 'heic', 'avif'],
	archive: ['zip', 'rar', '7z', 'tar', 'gz', 'tgz', 'bz2', 'xz', 'zst'],
	document: [
		'pdf',
		'epub',
		'txt',
		'md',
		'rtf',
		'doc',
		'docx',
		'odt',
		'xls',
		'xlsx',
		'ods',
		'csv',
		'ppt',
		'pptx',
		'odp'
	],
	// Installers and disk images.
	program: [
		'exe',
		'msi',
		'msix',
		'dmg',
		'pkg',
		'deb',
		'rpm',
		'appimage',
		'flatpak',
		'apk',
		'iso',
		'img'
	]
};

const KIND_OF = new Map<string, FileKind>(
	Object.entries(EXTENSIONS).flatMap(([kind, list]) =>
		list.map((ext) => [ext, kind as FileKind] as const)
	)
);

/** By the last extension: `film.tar.gz` is an archive. */
export function fileKind(name: string): FileKind {
	const dot = name.lastIndexOf('.');
	if (dot <= 0) {
		return 'other';
	}
	return KIND_OF.get(name.slice(dot + 1).toLowerCase()) ?? 'other';
}
