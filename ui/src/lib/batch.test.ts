import { describe, expect, it } from 'vitest';
import type { FoundLink } from '@ottie/api-types/FoundLink';
import {
	afterAdding,
	choiceOf,
	chooseForAll,
	chosenRows,
	holdsSeveral,
	requests,
	rowNote,
	rowsOf,
	textOf,
	tickAll
} from './batch';
import { plain } from './testing';

function link(name: string, known = false): FoundLink {
	return {
		kind: 'link',
		url: `https://example.com/${name}`,
		name,
		matched: { category: 1, extension: 'zip', byFolder: false },
		known
	};
}

const ftp: FoundLink = {
	kind: 'invalid',
	text: 'ftp://example.com/c.zip',
	refused: { error: '…', code: 'unsupportedScheme', scheme: 'ftp' }
};

describe('batch', () => {
	it('opens the list only for more than one link', () => {
		expect(holdsSeveral('https://example.com/a.zip')).toBe(false);
		expect(holdsSeveral('https://example.com/r?to=https://example.com/a.zip')).toBe(false);
		expect(holdsSeveral('see https://example.com/a.zip')).toBe(false);
		expect(holdsSeveral('https://example.com/a.zip\nhttps://example.com/b.zip')).toBe(true);
		expect(holdsSeveral('a: https://example.com/a.zip, b: http://example.com/b.zip')).toBe(true);
	});

	it('opens the list for a link with a sequence', () => {
		for (const sequence of [
			'[1-3]',
			'[001-120]',
			'[a-z]',
			'[A-C:2]',
			'[0-100:10]',
			'{a,b}',
			'{,_hd}'
		]) {
			expect(holdsSeveral(`https://example.com/img${sequence}.jpg`), sequence).toBe(true);
		}
		for (const notOne of ['[::1]', '[]', '[0]', '[a-Z]', '{id}', '\\[1-3]', '\\{a,b}']) {
			expect(holdsSeveral(`https://example.com/a${notOne}`), notOne).toBe(false);
		}
		expect(holdsSeveral('http://[::1]:8080/a.zip')).toBe(false);
		expect(holdsSeveral('[1-3] is not a link')).toBe(false);
	});

	it('ticks the links that can be added and aren’t listed yet', () => {
		const rows = rowsOf([link('a.zip'), link('b.zip', true), ftp], [], 'auto');
		expect(rows.map((row) => row.chosen)).toEqual([true, false, false]);
		expect(rows.every((row) => row.choice === 'auto' && row.refused === null)).toBe(true);
		expect(chosenRows(rows).map((row) => row.found)).toEqual([link('a.zip')]);
	});

	it('keeps how each link was set when the text changes', () => {
		let rows = rowsOf([link('a.zip'), link('b.zip')], [], 'auto');
		rows = [{ ...rows[0], chosen: false, choice: 3 }, rows[1]];
		const again = rowsOf([link('c.zip'), link('a.zip')], rows, 5);
		expect(again.map((row) => [row.found, row.chosen, row.choice])).toEqual([
			[link('c.zip'), true, 5],
			[link('a.zip'), false, 3]
		]);
	});

	it('reads a category menu', () => {
		expect(choiceOf('auto')).toBe('auto');
		expect(choiceOf('3')).toBe(3);
		expect(choiceOf('')).toBe('auto');
	});

	it('sets the category of all, and ticks all or none', () => {
		const rows = rowsOf([link('a.zip'), link('b.zip', true), ftp], [], 'auto');
		expect(chooseForAll(rows, 4).map((row) => row.choice)).toEqual([4, 4, 4]);
		expect(tickAll(rows, true).map((row) => row.chosen)).toEqual([true, true, false]);
		expect(tickAll(rows, false).map((row) => row.chosen)).toEqual([false, false, false]);
	});

	it('asks for the chosen links, with their category and the folder', () => {
		const rows = rowsOf([link('a.zip'), link('b.zip')], [], 'auto');
		rows[1].choice = 3;
		expect(requests(chosenRows(rows), ' /dl ')).toEqual([
			{ url: 'https://example.com/a.zip', destDir: '/dl' },
			{ url: 'https://example.com/b.zip', category: 3, destDir: '/dl' }
		]);
		expect(requests(chosenRows(rows), '')).toEqual([
			{ url: 'https://example.com/a.zip' },
			{ url: 'https://example.com/b.zip', category: 3 }
		]);
	});

	it('keeps only the refused links once added, saying why', () => {
		const rows = rowsOf([link('a.zip'), link('b.zip'), link('c.zip', true), ftp], [], 'auto');
		const sent = chosenRows(rows);
		const left = afterAdding(
			rows,
			sent,
			{
				ids: ['2026.10.14-10.00.00-1', null],
				refused: [{ index: 1, error: '…', code: 'noFolder', category: 'General' }]
			},
			'en'
		);
		expect(left.map((row) => row.found)).toEqual([link('b.zip'), link('c.zip', true), ftp]);
		expect(left[0].chosen).toBe(false);
		expect(left[0].refused).toBe('Choose a folder: the category General has none.');
		expect(textOf(left)).toBe(
			'https://example.com/b.zip\nhttps://example.com/c.zip\nftp://example.com/c.zip'
		);
	});

	it('says what’s wrong with a link', () => {
		const [fine, known, invalid] = rowsOf([link('a.zip'), link('b.zip', true), ftp], [], 'auto');
		expect(rowNote(fine, 'en')).toBeNull();
		expect(rowNote(known, 'en')).toBe('Already in the list');
		expect(rowNote(invalid, 'en')).toBe('Only http and https links are supported (not ftp).');
		expect(plain(rowNote(known, 'fr') ?? '')).toBe('Déjà dans la liste');
		expect(rowNote({ ...fine, refused: 'No.' }, 'en')).toBe('No.');
	});
});
