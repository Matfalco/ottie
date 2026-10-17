import { describe, expect, it } from 'vitest';
import { RecentPosts } from './posts';

describe('RecentPosts', () => {
	it('remembers a posted link for a minute', () => {
		const posts = new RecentPosts();
		posts.note('https://example.com/export', 1000);
		expect(posts.has('https://example.com/export', 1000 + 60_000)).toBe(true);
		expect(posts.has('https://example.com/export', 1001 + 60_000)).toBe(false);
		expect(posts.has('https://example.com/other', 1000)).toBe(false);
	});

	it('forgets the oldest past 200', () => {
		const posts = new RecentPosts();
		for (let i = 0; i <= 200; i++) {
			posts.note(`https://example.com/${i}`, 1000 + i);
		}
		expect(posts.has('https://example.com/0', 1300)).toBe(false);
		expect(posts.has('https://example.com/200', 1300)).toBe(true);
	});
});
