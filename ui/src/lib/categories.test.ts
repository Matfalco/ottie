import { describe, expect, it } from 'vitest';
import {
	applyCategoryEvent,
	categoryName,
	describeLimits,
	fieldToLimit,
	limitToField,
	parseExtensions,
	sorted,
	suggestFolder,
	type Categories
} from './categories';
import { category, plain } from './testing';

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

	it('show limits in kB/s or MB/s, and read them back', () => {
		expect(limitToField(0)).toEqual({ value: '', unit: 'MB' });
		expect(limitToField(1.5 * 1024 * 1024)).toEqual({ value: '1.5', unit: 'MB' });
		expect(limitToField(300 * 1024)).toEqual({ value: '300', unit: 'kB' });
		expect(fieldToLimit('1,5', 'MB')).toBe(1.5 * 1024 * 1024);
		expect(fieldToLimit('', 'kB')).toBe(0);
		expect(fieldToLimit('-3', 'kB')).toBe(0);
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
