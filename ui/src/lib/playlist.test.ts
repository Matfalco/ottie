import { describe, expect, it } from 'vitest';
import { playlist, playlistName } from './playlist';

describe('playlist', () => {
	it('holds the one link', () => {
		const link = 'http://127.0.0.1:24068/api/v1/stream/9f2c/My%20Film.mkv';
		expect(playlist(link)).toBe(`#EXTM3U\n${link}\n`);
	});

	it('is named after the download', () => {
		expect(playlistName('My Film.mkv')).toBe('My Film.m3u');
		expect(playlistName('archive.tar.gz')).toBe('archive.tar.m3u');
		expect(playlistName('README')).toBe('README.m3u');
		expect(playlistName('.hidden')).toBe('.hidden.m3u');
		expect(playlistName('')).toBe('ottie.m3u');
	});
});
