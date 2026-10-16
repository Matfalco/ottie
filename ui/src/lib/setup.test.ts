import { describe, expect, it } from 'vitest';
import {
	asCategory,
	edited,
	folderOf,
	notMade,
	startAt,
	starters,
	stepsHere,
	toCreate
} from './setup';
import { category } from './testing';

describe('setup', () => {
	it('has the desktop app’s steps only in the desktop app', () => {
		expect(stepsHere(false)).toEqual(['look', 'downloads', 'categories']);
		expect(stepsHere(true)).toEqual([
			'look',
			'downloads',
			'categories',
			'startup',
			'clipboard',
			'notifications'
		]);
	});

	it('carries on after the last step someone went past', () => {
		const here = stepsHere(true);
		expect(startAt(here, null)).toBe(0);
		expect(startAt(here, 'downloads')).toBe(2);
		expect(startAt(here, 'notifications')).toBe(5);
		// Past the desktop app's steps in a browser: the last one here.
		expect(startAt(stepsHere(false), 'startup')).toBe(2);
		expect(startAt(stepsHere(false), 'look')).toBe(1);
	});

	it('offers only the starters no category is named after', () => {
		const made = [category(), category({ id: 2, name: 'MUSIC' })];
		const left = notMade(starters('en'), made, 'en');
		expect(left.map((s) => s.body.name)).toEqual([
			'Videos',
			'Images',
			'Archives',
			'Documents',
			'Programs'
		]);
	});

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
