import { describe, expect, it } from 'vitest';
import { asCategory, edited, folderOf, starters, toCreate } from './setup';
import { category } from './testing';

describe('setup', () => {
	it('offers a ticked category per kind of file, in the page language', () => {
		const list = starters('fr');
		expect(list.map((s) => s.body.name)).toEqual([
			'Vidéos',
			'Musique',
			'Images',
			'Archives',
			'Documents',
			'Programmes'
		]);
		expect(list.every((s) => s.chosen)).toBe(true);
		expect(list[0].body.extensions).toContain('mkv');
	});

	it('puts each in a subfolder of the downloads folder, which it follows', () => {
		const [videos] = starters('en');
		expect(folderOf(videos, 'D:\\Downloads')).toBe('D:\\Downloads\\Videos');
		expect(folderOf(videos, '/home/me/Downloads/')).toBe('/home/me/Downloads/Videos');
		expect(folderOf(videos, '')).toBeNull();
		expect(asCategory(videos, '/dl')).toMatchObject({ name: 'Videos', folder: '/dl/Videos' });
	});

	it('keeps following after a rename, but not after the folder is changed', () => {
		const [videos] = starters('en');
		const renamed = edited(videos, { ...videos.body, name: 'Films', folder: '/dl/Films' }, '/dl');
		expect(folderOf(renamed, '/data')).toBe('/data/Films');
		const moved = edited(videos, { ...videos.body, folder: '/media/films' }, '/dl');
		expect(folderOf(moved, '/data')).toBe('/media/films');
	});

	it('creates the ticked ones whose name is free', () => {
		const list = starters('en');
		list[1].chosen = false; // Music
		const bodies = toCreate(list, '/dl', [category(), category({ id: 2, name: 'videos' })], 'en');
		expect(bodies.map((b) => b.name)).toEqual(['Images', 'Archives', 'Documents', 'Programs']);
		expect(bodies[0].folder).toBe('/dl/Images');
	});
});
