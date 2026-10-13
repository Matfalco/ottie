import { describe, expect, it } from 'vitest';
import { automaticLabel, defaultFolder, jobRequest, streamsByDefault } from './adding';
import type { Categories } from './categories';
import { category, plain } from './testing';

const categories: Categories = {
	1: category(),
	2: category({ id: 2, name: 'Other', folder: null, builtIn: 'other' }),
	3: category({ id: 3, name: 'Videos', folder: '/dl/Videos', extensions: ['mkv', 'mp4'] })
};

describe('adding', () => {
	it('says which category the file type matched, and why', () => {
		const videos = { category: 3, extension: 'mp4', byFolder: false };
		expect(automaticLabel(videos, categories, 'en')).toBe('Automatic: Videos (.mp4)');
		expect(plain(automaticLabel(videos, categories, 'fr'))).toBe('Automatique : Videos (.mp4)');
		const general = { category: 1, extension: 'php', byFolder: false };
		expect(automaticLabel(general, categories, 'en')).toBe('Automatic: General');
		expect(automaticLabel(general, categories, 'fr')).toContain('Général');
		expect(automaticLabel(null, categories, 'en')).toBe('Automatic');
	});

	it('says when the folder decided', () => {
		const inVideos = { category: 3, extension: 'txt', byFolder: true };
		expect(automaticLabel(inVideos, categories, 'en')).toBe('Automatic: Videos (its folder)');
		const elsewhere = { category: 2, extension: 'mp4', byFolder: true };
		expect(automaticLabel(elsewhere, categories, 'en')).toBe(
			'Automatic: Other (a folder of no category)'
		);
		expect(plain(automaticLabel(elsewhere, categories, 'fr'))).toBe(
			'Automatique : Autres (dossier d’aucune catégorie)'
		);
	});

	it('uses the folder of the chosen or matched category', () => {
		const matched = { category: 3, extension: 'mkv', byFolder: false };
		expect(defaultFolder('auto', matched, categories)).toBe('/dl/Videos');
		expect(defaultFolder(1, matched, categories)).toBe('/downloads');
		expect(defaultFolder('auto', null, categories)).toBeNull();
		expect(defaultFolder(9, matched, categories)).toBeNull();
	});

	it('sends a category only when the user picked one', () => {
		expect(jobRequest(' https://a.com/f.mkv ', 'auto', ' ')).toEqual({
			url: 'https://a.com/f.mkv'
		});
		expect(jobRequest('https://a.com/f', 3, ' /media ')).toEqual({
			url: 'https://a.com/f',
			category: 3,
			destDir: '/media'
		});
		expect(jobRequest('https://a.com/f', 'auto', '', false)).toEqual({
			url: 'https://a.com/f',
			streaming: false
		});
	});

	it('streams videos and music by default, if the settings say so', () => {
		const matched = (extension: string | null) => ({ category: 1, extension, byFolder: false });
		expect(streamsByDefault(matched('MKV'), true)).toBe(true);
		expect(streamsByDefault(matched('flac'), true)).toBe(true);
		expect(streamsByDefault(matched('zip'), true)).toBe(false);
		expect(streamsByDefault(matched(null), true)).toBe(false);
		expect(streamsByDefault(null, true)).toBe(false);
		expect(streamsByDefault(matched('mkv'), false)).toBe(false);
	});
});
