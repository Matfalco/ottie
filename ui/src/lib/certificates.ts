// FTPS servers' certificates trusted by hand (docs/decisions/0054): how a
// fingerprint is shown, and the one a download stopped at.

import type { Job } from '@ottie/api-types/Job';

/** A SHA-256 fingerprint as people compare them: `AB:CD:…`. */
export function formatFingerprint(fingerprint: string): string {
	return (fingerprint.toUpperCase().match(/.{1,2}/g) ?? []).join(':');
}

/** The certificate a download stopped at because nobody trusts it: the
 * panel offers to trust it. */
export function untrustedCertificate(job: Job): { host: string; fingerprint: string } | null {
	if (job.state !== 'failed' || job.problem?.code !== 'certificate') {
		return null;
	}
	return { host: job.problem.host, fingerprint: job.problem.fingerprint };
}
