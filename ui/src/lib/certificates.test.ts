import { describe, expect, it } from 'vitest';
import { formatFingerprint, untrustedCertificate } from './certificates';
import { job } from './testing';

describe('certificates', () => {
	it('shows a fingerprint in pairs', () => {
		expect(formatFingerprint('ab01cd')).toBe('AB:01:CD');
		expect(formatFingerprint('ab'.repeat(32))).toHaveLength(95);
	});

	it('offers to trust only a download stopped at a certificate', () => {
		const problem = { code: 'certificate', host: 'nas.local', fingerprint: 'ab' } as const;
		expect(untrustedCertificate(job({ state: 'failed', problem }))).toEqual({
			host: 'nas.local',
			fingerprint: 'ab'
		});
		expect(untrustedCertificate(job({ state: 'retrying', problem }))).toBeNull();
		expect(untrustedCertificate(job({ state: 'failed', problem: null }))).toBeNull();
	});
});
