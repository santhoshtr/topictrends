document.addEventListener("DOMContentLoaded", async () => {
	document.getElementById("delta-form").addEventListener("submit", onSubmit);

	await populateWikiDropdown();
	populateFormFromQueryParams();
});

async function onSubmit(event) {
	event.preventDefault();

	const params = new URLSearchParams();
	const wiki = document.getElementById("wiki").value;
	const baselineStartDate = document.getElementById(
		"baseline_start_date",
	).value;
	const baselineEndDate = document.getElementById("baseline_end_date").value;
	const impactStartDate = document.getElementById("impact_start_date").value;
	const impactEndDate = document.getElementById("impact_end_date").value;

	const depth = document.getElementById("depth").value;
	const limit = document.getElementById("limit").value;

	params.append("wiki", wiki);
	params.append("baseline_start_date", baselineStartDate);
	params.append("baseline_end_date", baselineEndDate);
	params.append("impact_start_date", impactStartDate);
	params.append("impact_end_date", impactEndDate);
	params.append("depth", depth);
	params.append("limit", limit);
	try {
		// Update the browser URL with the new parameters
		const newUrl = `${window.location.pathname}?${params.toString()}`;
		window.history.pushState({}, "", newUrl);

		const data = await fetchDeltaData(
			wiki,
			baselineStartDate,
			baselineEndDate,
			impactStartDate,
			impactEndDate,
			depth,
			limit,
		);
		if (data) {
			renderCategoryAccordions(data);
			const gainsCount = data.categories.filter(
				(c) => c.delta_percentage > 0,
			).length;
			const lossesCount = data.categories.filter(
				(c) => c.delta_percentage < 0,
			).length;
			showMessage(
				`Loaded ${data.categories.length} categories (${gainsCount} gains, ${lossesCount} losses)`,
				"success",
			);
		}
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch data. Please try again.", "error");
	}
}

async function fetchDeltaData(
	wiki,
	baselineStartDate,
	baselineEndDate,
	impactStartDate,
	impactEndDate,
	depth,
	limit,
) {
	const params = new URLSearchParams({
		wiki: wiki,
		baseline_start_date: baselineStartDate,
		baseline_end_date: baselineEndDate,
		impact_start_date: impactStartDate,
		impact_end_date: impactEndDate,
		depth: depth || 0,
		limit: limit || 100,
	});

	const API_URL = `https://topictrends.wmcloud.org/api/pageedits/delta/categories?${params.toString()}`;

	try {
		const response = await fetch(API_URL);
		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}
		const data = await response.json();
		return data;
	} catch (error) {
		console.error("Error fetching data:", error);
		showMessage(`Error loading data: ${error.message}`, "error");
	}
}

async function fetchArticleDeltaData(
	wiki,
	categoryQid,
	baselineStartDate,
	baselineEndDate,
	impactStartDate,
	impactEndDate,
	depth,
	limit,
) {
	const params = new URLSearchParams({
		wiki: wiki,
		category_qid: categoryQid,
		baseline_start_date: baselineStartDate,
		baseline_end_date: baselineEndDate,
		impact_start_date: impactStartDate,
		impact_end_date: impactEndDate,
		depth: depth || 0,
		limit: limit || 50,
	});

	const API_URL = `https://topictrends.wmcloud.org/api/pageedits/delta/articles?${params.toString()}`;

	try {
		const response = await fetch(API_URL);
		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}
		const data = await response.json();
		return data;
	} catch (error) {
		console.error("Error fetching articles data:", error);
		showMessage(`Error loading articles data: ${error.message}`, "error");
	}
}

/**
 * Renders category data as accordion sections (Gains/Losses)
 * @param {Object} data - API response with categories array
 */
function renderCategoryAccordions(data) {
	const gainsSection = document.getElementById("gains-section");
	const lossesSection = document.getElementById("losses-section");
	const gainsList = document.getElementById("gains-list");
	const lossesList = document.getElementById("losses-list");
	const emptyState = document.getElementById("empty-state");

	// Clear existing content
	gainsList.innerHTML = "";
	lossesList.innerHTML = "";

	// Separate categories into gains and losses
	const gains = data.categories.filter((cat) => cat.delta_percentage > 0);
	const losses = data.categories.filter((cat) => cat.delta_percentage < 0);

	// Sort by absolute delta (largest changes first)
	gains.sort((a, b) => b.delta_percentage - a.delta_percentage);
	losses.sort((a, b) => a.delta_percentage - b.delta_percentage);

	// Hide empty state if we have data
	if (gains.length > 0 || losses.length > 0) {
		emptyState.hidden = true;
	}

	// Render gains
	if (gains.length > 0) {
		gainsSection.style.display = "flex";
		const countSpan = gainsSection.querySelector(".section-count");
		countSpan.textContent = gains.length;

		for (const category of gains) {
			gainsList.appendChild(createCategoryAccordion(category, "positive"));
		}
	} else {
		gainsSection.style.display = "none";
	}

	// Render losses
	if (losses.length > 0) {
		lossesSection.style.display = "flex";
		const countSpan = lossesSection.querySelector(".section-count");
		countSpan.textContent = losses.length;

		for (const category of losses) {
			lossesList.appendChild(createCategoryAccordion(category, "negative"));
		}
	} else {
		lossesSection.style.display = "none";
	}
}

/**
 * Creates a single category accordion element
 * @param {Object} category - Category data from API
 * @param {string} type - 'positive' or 'negative'
 * @returns {HTMLDetailsElement} - <details> element
 */
function createCategoryAccordion(category, type) {
	const details = document.createElement("details");
	details.className = "category-accordion";
	details.name = type === "positive" ? "gains-accordion" : "losses-accordion";
	details.dataset.categoryQid = category.category_qid;
	details.dataset.categoryTitle = category.category_title;

	// Create summary (accordion header)
	const summary = document.createElement("summary");
	summary.className = "category-summary";

	// Category name
	const nameSpan = document.createElement("span");
	nameSpan.className = "category-name";
	nameSpan.textContent = category.category_title.replace(/_/g, " ");

	// Delta percentage
	const deltaDiv = document.createElement("div");
	deltaDiv.className = `category-delta ${type}`;
	const sign = category.delta_percentage >= 0 ? "+" : "";
	deltaDiv.textContent = `${sign}${category.delta_percentage.toFixed(2)}%`;

	// Edits (baseline → impact)
	const editsDiv = document.createElement("div");
	editsDiv.className = "category-edits";

	const editsLabel = document.createElement("span");
	editsLabel.className = "edits-label";
	editsLabel.textContent = "Edits";

	const editsRange = document.createElement("span");
	editsRange.className = "edits-range";
	editsRange.textContent = `${category.baseline_edits.toLocaleString()} → ${category.impact_edits.toLocaleString()}`;

	editsDiv.appendChild(editsLabel);
	editsDiv.appendChild(editsRange);

	// Plot button
	const plotLink = document.createElement("a");
	plotLink.className = "plot-button";
	plotLink.textContent = "📊";
	plotLink.title = "View trend chart";
	plotLink.target = "_blank";
	plotLink.rel = "noopener noreferrer";

	// Get wiki and depth from form
	const wiki = document.getElementById("wiki").value;
	const depth = document.getElementById("depth").value || "0";

	// Build plot URL with date range (today - 1 month to yesterday)
	const endDate = new Date();
	endDate.setDate(endDate.getDate() - 1); // Yesterday
	const startDate = new Date();
	startDate.setMonth(startDate.getMonth() - 1); // One month ago

	const formatDate = (date) => date.toISOString().split("T")[0];
	plotLink.href = `https://topictrends.wmcloud.org/?type=category&wiki=${wiki}&start_date=${formatDate(startDate)}&end_date=${formatDate(endDate)}&depth=${depth}&category=${category.category_title}`;

	// Prevent accordion toggle when clicking plot button
	plotLink.addEventListener("click", (e) => {
		e.stopPropagation();
	});

	// Assemble summary
	summary.appendChild(nameSpan);
	summary.appendChild(deltaDiv);
	summary.appendChild(editsDiv);
	summary.appendChild(plotLink);
	details.appendChild(summary);

	// Add event listener for lazy loading articles
	details.addEventListener("toggle", handleAccordionToggle);

	return details;
}

/**
 * Handles accordion expand/collapse
 * Lazy loads articles when expanded for the first time
 * @param {Event} event - Toggle event
 */
async function handleAccordionToggle(event) {
	const details = event.target;

	// Only load articles on first open
	if (details.open && !details.dataset.loaded) {
		const categoryQid = details.dataset.categoryQid;
		const categoryTitle = details.dataset.categoryTitle;

		// Create articles container
		const articlesContainer = document.createElement("div");
		articlesContainer.className = "articles-container";

		// Show loading indicator
		articlesContainer.innerHTML =
			'<div class="loading-indicator">Loading articles</div>';
		details.appendChild(articlesContainer);

		try {
			// Get form values
			const wiki = document.getElementById("wiki").value;
			const baselineStartDate = document.getElementById(
				"baseline_start_date",
			).value;
			const baselineEndDate =
				document.getElementById("baseline_end_date").value;
			const impactStartDate =
				document.getElementById("impact_start_date").value;
			const impactEndDate = document.getElementById("impact_end_date").value;
			const depth = document.getElementById("depth").value;

			const articlesData = await fetchArticleDeltaData(
				wiki,
				categoryQid,
				baselineStartDate,
				baselineEndDate,
				impactStartDate,
				impactEndDate,
				depth,
				100, // Fetch more articles (no pagination needed)
			);

			if (articlesData && articlesData.articles.length > 0) {
				renderArticles(articlesContainer, articlesData.articles, wiki);
				details.dataset.loaded = "true";

				showMessage(
					`Loaded ${articlesData.articles.length} articles for: ${categoryTitle}`,
					"success",
				);
			} else {
				articlesContainer.innerHTML =
					'<div class="no-data">No articles found in this category</div>';
			}
		} catch (error) {
			console.error("Error loading articles:", error);
			articlesContainer.innerHTML =
				'<div class="no-data">Error loading articles. Please try again.</div>';
			showMessage(`Failed to load articles for: ${categoryTitle}`, "error");
		}
	}
}

/**
 * Renders articles list inside accordion
 * @param {HTMLElement} container - Articles container element
 * @param {Array} articles - Array of article objects
 * @param {string} wiki - Wiki code for building Wikipedia links
 */
function renderArticles(container, articles, wiki) {
	container.innerHTML = "";

	// Separate into gains and losses
	const gains = articles.filter((art) => art.delta_percentage > 0);
	const losses = articles.filter((art) => art.delta_percentage < 0);

	// Sort by delta magnitude
	gains.sort((a, b) => b.delta_percentage - a.delta_percentage);
	losses.sort((a, b) => a.delta_percentage - b.delta_percentage);

	// Render gains section
	if (gains.length > 0) {
		const gainsHeader = document.createElement("div");
		gainsHeader.className = "articles-section-header";
		gainsHeader.innerHTML = "<span>📈</span><span>Article Gains</span>";
		container.appendChild(gainsHeader);

		for (const article of gains) {
			container.appendChild(createArticleElement(article, wiki));
		}
	}

	// Render losses section
	if (losses.length > 0) {
		const lossesHeader = document.createElement("div");
		lossesHeader.className = "articles-section-header";
		lossesHeader.innerHTML = "<span>📉</span><span>Article Losses</span>";
		container.appendChild(lossesHeader);

		for (const article of losses) {
			container.appendChild(createArticleElement(article, wiki));
		}
	}
}

/**
 * Creates a single article element with Wikipedia link
 * @param {Object} article - Article data from API
 * @param {string} wiki - Wiki code (e.g., 'enwiki')
 * @returns {HTMLElement} - Article div
 */
function createArticleElement(article, wiki) {
	const div = document.createElement("div");
	const type = article.delta_percentage >= 0 ? "positive" : "negative";
	div.className = `article-item ${type}`;

	// Article title as clickable link
	const titleLink = document.createElement("a");
	titleLink.className = "article-title";
	titleLink.textContent = article.article_title.replace(/_/g, " ");
	titleLink.href = buildWikipediaUrl(wiki, article.article_title);
	titleLink.target = "_blank";
	titleLink.rel = "noopener noreferrer";

	// Delta percentage
	const deltaSpan = document.createElement("span");
	deltaSpan.className = `article-delta ${type}`;
	const sign = article.delta_percentage >= 0 ? "+" : "";
	deltaSpan.textContent = `${sign}${article.delta_percentage.toFixed(2)}%`;

	// Edits (baseline → impact)
	const editsDiv = document.createElement("div");
	editsDiv.className = "article-edits";

	const editsLabel = document.createElement("span");
	editsLabel.className = "edits-label";
	editsLabel.textContent = "Edits";

	const editsValue = document.createElement("span");
	editsValue.textContent = `${article.baseline_edits.toLocaleString()} → ${article.impact_edits.toLocaleString()}`;

	editsDiv.appendChild(editsLabel);
	editsDiv.appendChild(editsValue);

	// Plot button
	const plotLink = document.createElement("a");
	plotLink.className = "plot-button";
	plotLink.textContent = "📊";
	plotLink.title = "View trend chart";
	plotLink.target = "_blank";
	plotLink.rel = "noopener noreferrer";

	// Build plot URL with date range (today - 1 month to yesterday)
	const endDate = new Date();
	endDate.setDate(endDate.getDate() - 1); // Yesterday
	const startDate = new Date();
	startDate.setMonth(startDate.getMonth() - 1); // One month ago

	const formatDate = (date) => date.toISOString().split("T")[0];
	plotLink.href = `https://topictrends.wmcloud.org/?type=article&wiki=${wiki}&start_date=${formatDate(startDate)}&end_date=${formatDate(endDate)}&article=${article.article_title}`;

	// Assemble article item
	div.appendChild(titleLink);
	div.appendChild(deltaSpan);
	div.appendChild(editsDiv);
	div.appendChild(plotLink);

	return div;
}

/**
 * Builds a Wikipedia article URL
 * @param {string} wiki - Wiki code (e.g., 'enwiki')
 * @param {string} title - Article title
 * @returns {string} - Full Wikipedia URL
 */
function buildWikipediaUrl(wiki, title) {
	// Extract language code from wiki (e.g., 'enwiki' -> 'en')
	const langCode = wiki.replace("wiki", "");

	// URL encode the title
	const encodedTitle = encodeURIComponent(title.replace(/ /g, "_"));

	return `https://${langCode}.wikipedia.org/wiki/${encodedTitle}`;
}

document.addEventListener("DOMContentLoaded", () => {
	const startDatePicker = document.getElementById("baseline_start_date");
	const endDatePicker = document.getElementById("baseline_end_date");
	const impactStartDatePicker = document.getElementById("impact_start_date");
	const impactEndDatePicker = document.getElementById("impact_end_date");

	const today = new Date();

	// Format the date to "YYYY-MM-DD" as required by the input type="date"
	let year = today.getFullYear();
	let month = String(today.getMonth() + 1).padStart(2, "0");
	let day = String(today.getDate()).padStart(2, "0");
	impactEndDatePicker.value = `${year}-${month}-${day}`;

	const twoMonthAgo = new Date(
		today.getFullYear(),
		today.getMonth() - 2,
		today.getDate(),
	);
	year = twoMonthAgo.getFullYear();
	month = String(twoMonthAgo.getMonth() + 1).padStart(2, "0");
	day = String(twoMonthAgo.getDate()).padStart(2, "0");

	startDatePicker.value = `${year}-${month}-${day}`;

	const oneMonthAgo = new Date(
		today.getFullYear(),
		today.getMonth() - 1,
		today.getDate(),
	);
	year = oneMonthAgo.getFullYear();
	month = String(oneMonthAgo.getMonth() + 1).padStart(2, "0");
	day = String(oneMonthAgo.getDate()).padStart(2, "0");

	endDatePicker.value = `${year}-${month}-${day}`;
	impactStartDatePicker.value = `${year}-${month}-${day}`;
});

function showMessage(message, type) {
	const messageEl = document.getElementById("status");
	messageEl.classList.remove("error-message");
	messageEl.classList.remove("success-message");
	messageEl.classList.remove("info-message");

	if (type === "error") {
		messageEl.classList.add("error-message");
	} else if (type === "success") {
		messageEl.classList.add("success-message");
	} else if (type === "info") {
		messageEl.classList.add("info-message");
	}

	messageEl.textContent = message;
}

function populateFormFromQueryParams() {
	const urlParams = new URLSearchParams(window.location.search);

	const wiki = urlParams.get("wiki");
	const baselineStartDate = urlParams.get("baseline_start_date");
	const baselineEndDate = urlParams.get("baseline_end_date");
	const impactStartDate = urlParams.get("impact_start_date");
	const impactEndDate = urlParams.get("impact_end_date");

	const depth = urlParams.get("depth");
	const limit = urlParams.get("limit");
	if (depth) {
		document.getElementById("depth").value = depth;
	}
	if (limit) {
		document.getElementById("limit").value = limit;
	}
	if (baselineStartDate) {
		document.getElementById("baseline_start_date").value = baselineStartDate;
	}
	if (baselineEndDate) {
		document.getElementById("baseline_end_date").value = baselineEndDate;
	}
	if (impactStartDate) {
		document.getElementById("impact_start_date").value = impactStartDate;
	}
	if (impactEndDate) {
		document.getElementById("impact_end_date").value = impactEndDate;
	}

	if (wiki) {
		document.getElementById("wiki").value = wiki;
		onSubmit(new Event("submit"));
	}
}

async function populateWikiDropdown() {
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
