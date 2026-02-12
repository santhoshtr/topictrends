/**
 * Wikipedia and wiki-related utilities
 */

/**
 * Populate the wiki dropdown with available wikis from wikis.json
 * @returns {Promise<void>}
 */
export async function populateWikiDropdown() {
	try {
		const response = await fetch("/static/wikis.json");
		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}

		const wikis = await response.json();
		const wikiSelect = document.getElementById("wiki");

		wikiSelect.innerHTML = "";

		wikis.forEach((wiki) => {
			const option = document.createElement("option");
			option.value = wiki.code;
			const displayName = `${wiki.langcode} - ${wiki.name}`;
			option.textContent = displayName;
			wikiSelect.appendChild(option);
		});

		console.log(`Loaded ${wikis.length} wikis to dropdown`);
	} catch (error) {
		console.error("Failed to load wiki list:", error);
		console.log("📋 Using fallback wiki list");
	}
}

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
