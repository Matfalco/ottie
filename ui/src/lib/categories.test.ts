import { describe, expect, it } from 'vitest';
import {
	applyCategoryEvent,
	categoryHue,
	categoryName,
	describeLimits,
	parseExtensions,
	someExtensions,
	sorted,
	suggestFolder,
	type Categories
} from './categories';
import { category, plain } from './testing';

describe('category tiles', () => {
	it('give the built-in ones their own colour and the others one by ID', () => {
		expect(categoryHue(category())).toBe('var(--accent)');
		expect(categoryHue(category({ id: 2, builtIn: 'other' }))).toBe('var(--faint)');
		expect(categoryHue(category({ id: 3 }))).toBe('var(--hue-4)');
		expect(categoryHue(category({ id: 9 }))).toBe('var(--hue-4)');
		expect(categoryHue(category({ id: 6 }))).toBe('var(--hue-1)');
	});

	it('show a few extensions and count the rest', () => {
		expect(someExtensions(['mp4', 'mkv'], 6)).toEqual([['mp4', 'mkv'], 0]);
		expect(someExtensions(['a', 'b', 'c', 'd'], 2)).toEqual([['a', 'b'], 2]);
	});
});

describe('categories', () => {
	it('follow events, General first then by name, Other last', () => {
		const categories: Categories = {
			1: category(),
			9: category({ id: 9, name: 'Other', builtIn: 'other' })
		};
		applyCategoryEvent(categories, {
			type: 'categoryChanged',
			category: category({ id: 3, name: 'Zips' })
		});
		applyCategoryEvent(categories, {
			type: 'categoryChanged',
			category: category({ id: 2, name: 'Music' })
		});
		expect(sorted(categories, 'en').map((c) => c.name)).toEqual([
			'General',
			'Music',
			'Zips',
			'Other'
		]);
		applyCategoryEvent(categories, { type: 'categoryRemoved', id: 3 });
		expect(Object.keys(categories)).toEqual(['1', '2', '9']);
	});

	it('read extensions however they are separated', () => {
		expect(parseExtensions(' mp4, .MKV;webm  avi ')).toEqual(['mp4', '.MKV', 'webm', 'avi']);
		expect(parseExtensions('  ')).toEqual([]);
	});

	it('describe their limits in each language', () => {
		expect(describeLimits(category(), 'en')).toBe('Any number at once · No speed limit');
		const capped = category({ maxConcurrent: 2, speedLimit: 1024 * 1024 });
		expect(plain(describeLimits(capped, 'fr'))).toBe('2 à la fois au plus · 1,0 Mo/s au plus');
	});

	it('suggest a subfolder of General’s folder', () => {
		expect(suggestFolder('C:\\Users\\me\\Downloads', 'Videos')).toBe(
			'C:\\Users\\me\\Downloads\\Videos'
		);
		expect(suggestFolder('/home/me/Downloads/', ' Music ')).toBe('/home/me/Downloads/Music');
		expect(suggestFolder('/home/me/Downloads', '')).toBe('/home/me/Downloads');
		expect(suggestFolder(null, 'Videos')).toBe('');
	});

	it('show the built-in name in the page’s language, unless renamed', () => {
		expect(categoryName(category(), 'fr')).toBe('Général');
		expect(categoryName(category({ name: 'Misc' }), 'fr')).toBe('Misc');
		expect(categoryName(category({ id: 2, name: 'General' }), 'fr')).toBe('General');
		const other = category({ id: 2, name: 'Other', builtIn: 'other' });
		expect(categoryName(other, 'fr')).toBe('Autres');
		expect(categoryName({ ...other, name: 'Elsewhere' }, 'fr')).toBe('Elsewhere');
	});
});
