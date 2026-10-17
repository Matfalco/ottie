import { m } from '#ui/paraglide/messages.js';
import { start } from '../pages/start';
import Options from './Options.svelte';

void start(Options, () => m.ext_title());
