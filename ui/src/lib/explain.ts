// The daemon's codes in the page's language. The daemon also sends English
// text, used for codes this UI doesn't know (a newer daemon) and as detail.

import type { ApiError } from '@ottie/api-types/ApiError';
import type { Problem } from '@ottie/api-types/Problem';
import { formatBytes, formatDuration } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** Why a download failed or is retrying. */
export function explainProblem(
	problem: Problem | null,
	english: string | null,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	if (!problem) {
		return english ?? '';
	}
	switch (problem.code) {
		case 'network':
			return m.problem_network({ detail: problem.detail }, options);
		case 'redirect':
			return m.problem_redirect({ detail: problem.detail }, options);
		case 'httpStatus':
			return httpStatus(problem.status, locale);
		case 'stalled':
			return m.problem_stalled({ time: formatDuration(problem.seconds, locale) }, options);
		case 'incomplete':
			return m.problem_incomplete(
				{
					received: formatBytes(problem.received, locale),
					expected: formatBytes(problem.expected, locale)
				},
				options
			);
		case 'remoteChanging':
			return m.problem_remote_changing({}, options);
		case 'disk':
			return m.problem_disk({ detail: problem.detail }, options);
		case 'other':
			return problem.detail;
		default:
			return english ?? '';
	}
}

function httpStatus(status: number, locale: Locale): string {
	const options = { locale };
	switch (status) {
		case 403:
			return m.problem_http_403({}, options);
		case 404:
			return m.problem_http_404({}, options);
		case 410:
			return m.problem_http_410({}, options);
		case 429:
			return m.problem_http_429({}, options);
		default:
			return status >= 500
				? m.problem_http_5xx({ status: String(status) }, options)
				: m.problem_http({ status: String(status) }, options);
	}
}

/** Why the daemon refused a request. `fallback` is used without a body. */
export function explainRefusal(
	body: ApiError | null,
	fallback: string,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	if (!body) {
		return fallback;
	}
	switch (body.code) {
		case 'invalidUrl':
			return m.reason_invalid_url({}, options);
		case 'unsupportedScheme':
			return m.reason_unsupported_scheme({ scheme: body.scheme }, options);
		case 'relativeFolder':
			return m.reason_relative_folder({}, options);
		case 'nonUtf8Path':
			return m.reason_non_utf8_path({}, options);
		case 'noFolder':
			return m.reason_no_folder({ category: body.category }, options);
		case 'jobNotFound':
			return m.reason_job_not_found({}, options);
		case 'invalidTransition':
			return m.reason_invalid_transition({}, options);
		case 'stopping':
			return m.reason_stopping({}, options);
		default:
			// Codes for screens this UI doesn't have yet, internal errors, and
			// codes from a newer daemon.
			return body.error;
	}
}
