import { describe, expect, it } from 'vitest';
import type { ApiError } from '@ottie/api-types/ApiError';
import type { Problem } from '@ottie/api-types/Problem';
import { explainProblem, explainRefusal } from './explain';
import { plain } from './testing';

describe('explainProblem', () => {
	it('translates each code, with its details', () => {
		const say = (problem: Problem, locale: 'en' | 'fr' = 'en') =>
			plain(explainProblem(problem, 'English text', locale));
		expect(say({ code: 'httpStatus', status: 403 })).toBe('The server refused access (403)');
		expect(say({ code: 'httpStatus', status: 502 })).toBe('The server has a problem (502)');
		expect(say({ code: 'httpStatus', status: 418 })).toBe('The server answered 418');
		expect(say({ code: 'stalled', seconds: 60 }, 'fr')).toBe(
			'Aucune donnée du serveur depuis 1 min'
		);
		expect(say({ code: 'incomplete', received: 1024, expected: 2048 })).toBe(
			'The connection closed early (1.0 kB of 2.0 kB)'
		);
		expect(say({ code: 'network', detail: 'connection refused' }, 'fr')).toBe(
			'Erreur réseau (connection refused)'
		);
		expect(say({ code: 'other', detail: 'something' })).toBe('something');
	});

	it('falls back to the English text', () => {
		expect(explainProblem(null, 'server said no', 'fr')).toBe('server said no');
		// A code from a newer daemon.
		const unknown = { code: 'quantumFlux' } as unknown as Problem;
		expect(explainProblem(unknown, 'flux!', 'fr')).toBe('flux!');
	});
});

describe('explainRefusal', () => {
	it('translates known codes and falls back to the English text', () => {
		const noFolder: ApiError = { error: '…', code: 'noFolder', category: 'General' };
		expect(plain(explainRefusal(noFolder, '', 'fr'))).toBe(
			'Choisissez un dossier : la catégorie General n’en a pas.'
		);
		const internal: ApiError = { error: 'database error: disk I/O', code: 'internal' };
		expect(explainRefusal(internal, '', 'fr')).toBe('database error: disk I/O');
		expect(explainRefusal(null, 'The daemon answered 502.', 'fr')).toBe('The daemon answered 502.');
	});

	it('names the setting out of range, with its range', () => {
		const say = (setting: string, min: number, max: number, locale: 'en' | 'fr') => {
			const body: ApiError = { error: '…', code: 'outOfRange', setting, min, max };
			return plain(explainRefusal(body, '', locale));
		};
		expect(say('maxConcurrent', 1, 50, 'en')).toBe('Downloads at once must be from 1 to 50.');
		expect(say('firstRetryWaitMs', 500, 3_600_000, 'fr')).toBe(
			'Première attente : de 0,5 à 3 600 secondes.'
		);
		expect(say('somethingNew', 0, 9, 'en')).toBe('somethingNew must be from 0 to 9.');
	});
});
