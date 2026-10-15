// Which language the page shows (docs/decisions/0041), set up before anything
// asks for it. Paraglide saves the first language it finds through its
// strategies' `setLocale`, which would fix the system's language for good;
// so these save nothing, and `switchLanguage` keeps the choice itself.

import { chosenLanguage, systemLanguage } from '#lib/languages.js';
import { defineCustomClientStrategy } from '#lib/paraglide/runtime.js';

defineCustomClientStrategy('custom-chosen', {
	getLocale: () => chosenLanguage() ?? undefined,
	setLocale: () => {}
});

defineCustomClientStrategy('custom-system', {
	getLocale: () => systemLanguage(),
	setLocale: () => {}
});
