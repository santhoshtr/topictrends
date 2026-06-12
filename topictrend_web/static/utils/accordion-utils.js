/**
 * Accordion rendering utilities for delta analysis views
 */

import { MAX_ACCORDION_ITEMS } from "./constants.js";
import { formatDateToISO, getMonthsAgo, getYesterday } from "./date-utils.js";
import { showMessage } from "./ui-utils.js";

/**
 * Create a category accordion element
 * @param {Object} category - Category data from API
 * @param {string} type - 'positive' or 'negative'
 * @param {Object} config - Configuration object
 * @param {string} config.metricName - Name of metric (e.g., 'views', 'edits')
 * @param {string} config.baselineKey - Key for baseline value (e.g., 'baseline_views')
 * @param {string} config.impactKey - Key for impact value (e.g., 'impact_views')
 * @param {string} config.trendsUrl - URL for trends page (e.g., '/pageviews/trends')
 * @returns {HTMLDetailsElement} Accordion element
 */
export function createCategoryAccordion(category, type, config) {
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

	// Metric value (views/edits baseline → impact)
	const metricDiv = document.createElement("div");
	metricDiv.className = `category-${config.metricName}`;

	if (config.metricName === "edits") {
		const metricLabel = document.createElement("span");
		metricLabel.className = `${config.metricName}-label`;
		metricLabel.textContent = "Edits";
		metricDiv.appendChild(metricLabel);
	}

	const metricRange = document.createElement("span");
	metricRange.className = `${config.metricName}-range`;
	metricRange.textContent = `${category[config.baselineKey].toLocaleString()} → ${category[config.impactKey].toLocaleString()}`;

	metricDiv.appendChild(metricRange);

	// Plot button
	const plotLink = createPlotButton(category, config.trendsUrl);

	// Assemble summary
	summary.appendChild(nameSpan);
	summary.appendChild(deltaDiv);
	summary.appendChild(metricDiv);
	summary.appendChild(plotLink);
	details.appendChild(summary);

	return details;
}

/**
 * Create plot button for accordion
 * @param {Object} category - Category data
 * @param {string} trendsUrl - Base trends URL
 * @returns {HTMLAnchorElement} Plot button link
 */
function createPlotButton(category, trendsUrl) {
	const plotLink = document.createElement("a");
	plotLink.className = "plot-button";
	plotLink.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
	plotLink.title = "View trend chart";
	plotLink.target = "_blank";
	plotLink.rel = "noopener noreferrer";

	// Get wiki from form
	const wiki = document.getElementById("wiki").value;

	// Build plot URL with date range (1 month ago to yesterday)
	const endDate = getYesterday();
	const startDate = getMonthsAgo(1);

	plotLink.href = `${trendsUrl}?type=category&wiki=${wiki}&start_date=${formatDateToISO(startDate)}&end_date=${formatDateToISO(endDate)}&category=${category.category_title}`;

	// Prevent accordion toggle when clicking plot button
	plotLink.addEventListener("click", (e) => {
		e.stopPropagation();
	});

	return plotLink;
}

/**
 * Render category accordions separated into gains and losses
 * @param {Object} data - Delta data with categories array
 * @param {Object} config - Configuration object
 */
export function renderCategoryAccordions(data, config) {
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
			gainsList.appendChild(
				createCategoryAccordion(category, "positive", config),
			);
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
			lossesList.appendChild(
				createCategoryAccordion(category, "negative", config),
			);
		}
	} else {
		lossesSection.style.display = "none";
	}
}

/**
 * Create accordion toggle handler for lazy loading articles
 * @param {Function} fetchArticlesFn - Function to fetch article data
 * @param {Function} renderArticlesFn - Function to render articles
 * @returns {Function} Event handler
 */
export function createAccordionToggleHandler(
	fetchArticlesFn,
	renderArticlesFn,
) {
	return async function handleAccordionToggle(event) {
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

				const articlesData = await fetchArticlesFn(
					wiki,
					categoryQid,
					baselineStartDate,
					baselineEndDate,
					impactStartDate,
					impactEndDate,
					MAX_ACCORDION_ITEMS,
				);

				if (articlesData && articlesData.articles.length > 0) {
					renderArticlesFn(articlesContainer, articlesData.articles, wiki);
					details.dataset.loaded = "true";

					showMessage(
						`Loaded ${articlesData.articles.length} articles for: ${categoryTitle}`,
						"success",
					);
				} else {
					articlesContainer.innerHTML =
						'<div class="empty-articles">No articles found</div>';
				}
			} catch (error) {
				console.error("Failed to load articles:", error);
				articlesContainer.innerHTML =
					'<div class="error-articles">Failed to load articles</div>';
			}
		}
	};
}
