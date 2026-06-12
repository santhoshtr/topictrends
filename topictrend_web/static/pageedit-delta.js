import {
	createCategoryAccordion as createBaseAccordion,
	renderCategoryAccordions as renderBaseAccordions,
} from "./utils/accordion-utils.js";
import { formatDateToISO } from "./utils/date-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import { buildWikipediaUrl } from "./utils/wiki-utils.js";
import "./components/wiki-selector.js";

// Configuration for pageedit delta accordions
const ACCORDION_CONFIG = {
	metricName: "edits",
	baselineKey: "baseline_edits",
	impactKey: "impact_edits",
	trendsUrl: "/pageedits/trends",
};

document.addEventListener("DOMContentLoaded", () => {
	document.getElementById("delta-form").addEventListener("submit", onSubmit);

	document
		.getElementById("delta-form")
		.addEventListener("form-fill-complete", () => {
			onSubmit(new Event("submit"));
		});

	if (!window.location.search) {
		document.querySelector(".examples").hidden = false;
	}
});

async function onSubmit(event) {
	event.preventDefault();

	document.querySelector(".examples").hidden = true;

	const params = new URLSearchParams();
	const wiki = document.getElementById("wiki").value;
	const baselineStartDate = document.getElementById(
		"baseline_start_date",
	).value;
	const baselineEndDate = document.getElementById("baseline_end_date").value;
	const impactStartDate = document.getElementById("impact_start_date").value;
	const impactEndDate = document.getElementById("impact_end_date").value;
	const limit = document.getElementById("limit").value;

	params.append("wiki", wiki);
	params.append("baseline_start_date", baselineStartDate);
	params.append("baseline_end_date", baselineEndDate);
	params.append("impact_start_date", impactStartDate);
	params.append("impact_end_date", impactEndDate);
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
	limit,
) {
	const params = new URLSearchParams({
		wiki: wiki,
		baseline_start_date: baselineStartDate,
		baseline_end_date: baselineEndDate,
		impact_start_date: impactStartDate,
		impact_end_date: impactEndDate,
		limit: limit || 100,
	});

	const API_URL = `/api/pageedits/delta/categories?${params.toString()}`;

	try {
		showProgress();
		const response = await fetch(API_URL);
		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}
		const data = await response.json();
		return data;
	} catch (error) {
		console.error("Error fetching data:", error);
		showMessage(`Error loading data: ${error.message}`, "error");
	} finally {
		hideProgress();
	}
}

async function fetchArticleDeltaData(
	wiki,
	categoryQid,
	baselineStartDate,
	baselineEndDate,
	impactStartDate,
	impactEndDate,
	limit,
) {
	const params = new URLSearchParams({
		wiki: wiki,
		category_qid: categoryQid,
		baseline_start_date: baselineStartDate,
		baseline_end_date: baselineEndDate,
		impact_start_date: impactStartDate,
		impact_end_date: impactEndDate,
		limit: limit || 50,
	});

	const API_URL = `/api/pageedits/delta/articles?${params.toString()}`;

	try {
		showProgress();
		const response = await fetch(API_URL);
		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}
		const data = await response.json();
		return data;
	} catch (error) {
		console.error("Error fetching articles data:", error);
		showMessage(`Error loading articles data: ${error.message}`, "error");
	} finally {
		hideProgress();
	}
}

/**
 * Renders category data as three tabs: Top / Trending up / Trending down
 * @param {Object} data - API response with categories array
 */
function renderCategoryAccordions(data) {
	const topList = document.getElementById("top-list");
	const gainsList = document.getElementById("gains-list");
	const lossesList = document.getElementById("losses-list");
	const emptyState = document.getElementById("empty-state");

	topList.innerHTML = "";
	gainsList.innerHTML = "";
	lossesList.innerHTML = "";

	const gains = data.categories.filter((c) => c.delta_percentage > 0);
	const losses = data.categories.filter((c) => c.delta_percentage < 0);

	gains.sort((a, b) => b.delta_percentage - a.delta_percentage);
	losses.sort((a, b) => a.delta_percentage - b.delta_percentage);

	if (gains.length > 0 || losses.length > 0) {
		emptyState.hidden = true;
	}

	// Top tab: all items sorted by |delta| descending
	const top = [...gains, ...losses].sort(
		(a, b) => Math.abs(b.delta_percentage) - Math.abs(a.delta_percentage),
	);
	for (const cat of top) {
		const type = cat.delta_percentage > 0 ? "positive" : "negative";
		topList.appendChild(createCategoryAccordion(cat, type, "top-accordion"));
	}

	// Trending up tab
	const tabUp = document.getElementById("tab-up");
	const labelUp = document.querySelector("label[for='tab-up']");
	if (gains.length > 0) {
		tabUp.hidden = false;
		labelUp.hidden = false;
		for (const cat of gains) {
			gainsList.appendChild(
				createCategoryAccordion(cat, "positive", "gains-accordion"),
			);
		}
	} else {
		tabUp.hidden = true;
		labelUp.hidden = true;
		if (tabUp.checked) document.getElementById("tab-top").checked = true;
	}

	// Trending down tab
	const tabDown = document.getElementById("tab-down");
	const labelDown = document.querySelector("label[for='tab-down']");
	if (losses.length > 0) {
		tabDown.hidden = false;
		labelDown.hidden = false;
		for (const cat of losses) {
			lossesList.appendChild(
				createCategoryAccordion(cat, "negative", "losses-accordion"),
			);
		}
	} else {
		tabDown.hidden = true;
		labelDown.hidden = true;
		if (tabDown.checked) document.getElementById("tab-top").checked = true;
	}
}

/**
 * Creates a single category accordion element
 * @param {Object} category - Category data from API
 * @param {string} type - 'positive' or 'negative'
 * @param {string} accordionName - name attribute for the <details> exclusive-open group
 * @returns {HTMLDetailsElement} - <details> element
 */
function createCategoryAccordion(category, type, accordionName) {
	const details = document.createElement("details");
	details.className = "category-accordion";
	details.name = accordionName;
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
	plotLink.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
	plotLink.title = "View trend chart";
	plotLink.target = "_blank";
	plotLink.rel = "noopener noreferrer";

	// Get wiki from form
	const wiki = document.getElementById("wiki").value;

	// Build plot URL with date range (today - 1 month to yesterday)
	const endDate = new Date();
	endDate.setDate(endDate.getDate() - 1); // Yesterday
	const startDate = new Date();
	startDate.setMonth(startDate.getMonth() - 1); // One month ago

	const formatDate = (date) => date.toISOString().split("T")[0];
	plotLink.href = `/pageedits/trends?type=category&wiki=${wiki}&start_date=${formatDate(startDate)}&end_date=${formatDate(endDate)}&category=${category.category_title}`;

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

			const articlesData = await fetchArticleDeltaData(
				wiki,
				categoryQid,
				baselineStartDate,
				baselineEndDate,
				impactStartDate,
				impactEndDate,
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
 * Renders articles list inside accordion as three tabs: Top / Trending up / Trending down
 * @param {HTMLElement} container - Articles container element
 * @param {Array} articles - Array of article objects
 * @param {string} wiki - Wiki code for building Wikipedia links
 */
function renderArticles(container, articles, wiki) {
	container.innerHTML = "";

	const gains = articles.filter((a) => a.delta_percentage > 0);
	const losses = articles.filter((a) => a.delta_percentage < 0);

	gains.sort((a, b) => b.delta_percentage - a.delta_percentage);
	losses.sort((a, b) => a.delta_percentage - b.delta_percentage);

	const top = [...gains, ...losses].sort(
		(a, b) => Math.abs(b.delta_percentage) - Math.abs(a.delta_percentage),
	);

	const details = container.closest("details");
	const scopeId = details
		? details.dataset.categoryQid.replace(/[^a-z0-9]/gi, "")
		: Date.now();
	const nameAttr = `art-tab-${scopeId}`;

	const radioTop = makeRadio(`art-top-${scopeId}`, nameAttr, true);
	const radioUp = makeRadio(`art-up-${scopeId}`, nameAttr, false);
	const radioDown = makeRadio(`art-down-${scopeId}`, nameAttr, false);

	const nav = document.createElement("nav");
	nav.className = "articles-tabs";

	const lblTop = makeTabLabel(`art-top-${scopeId}`, "Top");
	const lblUp = makeTabLabel(`art-up-${scopeId}`, "Trending up");
	const lblDown = makeTabLabel(`art-down-${scopeId}`, "Trending down");

	nav.appendChild(lblTop);
	nav.appendChild(lblUp);
	nav.appendChild(lblDown);

	const panelTop = makePanel(top, wiki);
	const panelUp = makePanel(gains, wiki);
	const panelDown = makePanel(losses, wiki);

	if (gains.length === 0) {
		radioUp.hidden = true;
		lblUp.hidden = true;
	}
	if (losses.length === 0) {
		radioDown.hidden = true;
		lblDown.hidden = true;
	}

	function showActivePanel() {
		panelTop.classList.toggle("active", radioTop.checked);
		panelUp.classList.toggle("active", radioUp.checked);
		panelDown.classList.toggle("active", radioDown.checked);
		lblTop.classList.toggle("active", radioTop.checked);
		lblUp.classList.toggle("active", radioUp.checked);
		lblDown.classList.toggle("active", radioDown.checked);
	}
	radioTop.addEventListener("change", showActivePanel);
	radioUp.addEventListener("change", showActivePanel);
	radioDown.addEventListener("change", showActivePanel);

	panelTop.classList.add("active");
	lblTop.classList.add("active");

	container.appendChild(radioTop);
	container.appendChild(radioUp);
	container.appendChild(radioDown);
	container.appendChild(nav);
	container.appendChild(panelTop);
	container.appendChild(panelUp);
	container.appendChild(panelDown);
}

function makeRadio(id, name, checked) {
	const input = document.createElement("input");
	input.type = "radio";
	input.id = id;
	input.name = name;
	input.checked = checked;
	input.hidden = true;
	return input;
}

function makeTabLabel(forId, text) {
	const label = document.createElement("label");
	label.setAttribute("for", forId);
	label.setAttribute("role", "tab");
	label.textContent = text;
	return label;
}

function makePanel(items, wiki) {
	const section = document.createElement("section");
	section.className = "articles-tab-panel";
	for (const article of items) {
		section.appendChild(createArticleElement(article, wiki));
	}
	return section;
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
	plotLink.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
	plotLink.title = "View trend chart";
	plotLink.target = "_blank";
	plotLink.rel = "noopener noreferrer";

	// Build plot URL with date range (today - 1 month to yesterday)
	const endDate = new Date();
	endDate.setDate(endDate.getDate() - 1); // Yesterday
	const startDate = new Date();
	startDate.setMonth(startDate.getMonth() - 1); // One month ago

	const formatDate = (date) => date.toISOString().split("T")[0];
	plotLink.href = `/pageedits/trends?type=article&wiki=${wiki}&start_date=${formatDate(startDate)}&end_date=${formatDate(endDate)}&article=${article.article_title}`;

	// Assemble article item
	div.appendChild(titleLink);

	const infoEl = document.createElement("wiki-article-info");
	infoEl.setAttribute("title", article.article_title);
	infoEl.setAttribute("wiki", wiki);
	div.appendChild(infoEl);

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
// Using imported buildWikipediaUrl from wiki-utils.js

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

// Using imported showMessage from ui-utils.js
