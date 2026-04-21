/**
 * Wikipedia and wiki-related utilities
 */

/**
 * Build a Wikipedia article URL from wiki code and title
 * @param {string} wiki - Wiki code (e.g., 'enwiki', 'mlwiki')
 * @param {string} title - Article title
 * @returns {string} Full Wikipedia URL
 */
export function buildWikipediaUrl(wiki, title) {
	// Extract language code from wiki (e.g., 'enwiki' -> 'en')
	const langCode = wiki.replace("wiki", "");

	// URL encode the title
	const encodedTitle = encodeURIComponent(title.replace(/ /g, "_"));

	return `https://${langCode}.wikipedia.org/wiki/${encodedTitle}`;
}

/**
 * Extract language code from wiki code
 * @param {string} wiki - Wiki code (e.g., 'enwiki', 'mlwiki')
 * @returns {string} Language code (e.g., 'en', 'ml')
 */
export function extractLangCode(wiki) {
	return wiki.replace("wiki", "");
}
