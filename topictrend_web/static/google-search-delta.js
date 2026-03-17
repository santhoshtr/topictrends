import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

document.addEventListener("DOMContentLoaded", async () => {
	document.getElementById("delta-form").addEventListener("submit", onSubmit);
	await populateWikiDropdown();
	populateFormFromQueryParams();
});

async function onSubmit(event) {
	event.preventDefault();
	document.querySelector(".examples").hidden = true;

	const wiki = document.getElementById("wiki").value;
	const baselineStartDate = document.getElementById(
		"baseline_start_date",
	).value;
	const baselineEndDate = document.getElementById("baseline_end_date").value;
	const impactStartDate = document.getElementById("impact_start_date").value;
	const impactEndDate = document.getElementById("impact_end_date").value;
	const depth = document.getElementById("depth").value;
	const limit = document.getElementById("limit").value;

	const params = new URLSearchParams({
		wiki,
		baseline_start_date: baselineStartDate,
		baseline_end_date: baselineEndDate,
		impact_start_date: impactStartDate,
		impact_end_date: impactEndDate,
		depth,
		limit,
	});

	window.history.pushState(
		{},
		"",
		`${window.location.pathname}?${params.toString()}`,
	);

	const data = await fetchCategoryDeltaData(params);
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
}

async function fetchCategoryDeltaData(params) {
	const url = `https://topictrends.wmcloud.org/api/googlesearch/delta/categories?${params.toString()}`;
	try {
		showProgress();
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}
		return await response.json();
	} catch (error) {
		console.error("Error fetching category delta data:", error);
		showMessage(`Error loading data: ${error.message}`, "error");
		return null;
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
	depth,
	limit,
) {
	const params = new URLSearchParams({
		wiki,
		category_qid: categoryQid,
		baseline_start_date: baselineStartDate,
		baseline_end_date: baselineEndDate,
		impact_start_date: impactStartDate,
		impact_end_date: impactEndDate,
		depth: depth || 0,
		limit: limit || 100,
	});

	const url = `https://topictrends.wmcloud.org/api/googlesearch/delta/articles?${params.toString()}`;
	const response = await fetch(url);
	if (!response.ok) {
		throw new Error(`HTTP error! status: ${response.status}`);
	}
	return await response.json();
}

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

	const top = [...gains, ...losses].sort(
		(a, b) => Math.abs(b.delta_percentage) - Math.abs(a.delta_percentage),
	);
	for (const category of top) {
		topList.appendChild(createCategoryAccordion(category, "top-accordion"));
	}
	for (const category of gains) {
		gainsList.appendChild(createCategoryAccordion(category, "gains-accordion"));
	}
	for (const category of losses) {
		lossesList.appendChild(
			createCategoryAccordion(category, "losses-accordion"),
		);
	}
}

function createCategoryAccordion(category, accordionName) {
	const details = document.createElement("details");
	details.className = "category-accordion";
	details.name = accordionName;
	details.dataset.categoryQid = category.category_qid;
	details.dataset.categoryTitle = category.category_title;

	const summary = document.createElement("summary");
	summary.className = "category-summary";

	const nameSpan = document.createElement("span");
	nameSpan.className = "category-name";
	nameSpan.textContent = category.category_title.replace(/_/g, " ");

	const deltaDiv = document.createElement("div");
	const type = category.delta_percentage >= 0 ? "positive" : "negative";
	deltaDiv.className = `category-delta ${type}`;
	const sign = category.delta_percentage >= 0 ? "+" : "";
	deltaDiv.textContent = `${sign}${category.delta_percentage.toFixed(2)}%`;

	const clicksDiv = document.createElement("div");
	clicksDiv.className = "category-views";
	const clicksLabel = document.createElement("span");
	clicksLabel.className = "views-label";
	clicksLabel.textContent = "Clicks";
	const clicksRange = document.createElement("span");
	clicksRange.className = "views-range";
	clicksRange.textContent = `${category.baseline_clicks.toLocaleString()} → ${category.impact_clicks.toLocaleString()}`;
	clicksDiv.appendChild(clicksLabel);
	clicksDiv.appendChild(clicksRange);

	const impressionsDiv = document.createElement("div");
	impressionsDiv.className = "category-views";
	const impressionsRange = document.createElement("span");
	impressionsRange.className = "views-range";
	impressionsRange.textContent = `Impressions: ${category.baseline_impressions.toLocaleString()} → ${category.impact_impressions.toLocaleString()}`;
	impressionsDiv.appendChild(impressionsRange);

	const plotLink = document.createElement("a");
	plotLink.className = "plot-button";
	plotLink.textContent = "📊";
	plotLink.title = "View trend chart";
	plotLink.target = "_blank";
	plotLink.rel = "noopener noreferrer";

	const wiki = document.getElementById("wiki").value;
	const depth = document.getElementById("depth").value || "0";
	const endDate = new Date();
	endDate.setDate(endDate.getDate() - 1);
	const startDate = new Date();
	startDate.setMonth(startDate.getMonth() - 1);
	const formatDate = (date) => date.toISOString().split("T")[0];
	plotLink.href = `/googlesearch/trends?type=category&wiki=${wiki}&start_date=${formatDate(startDate)}&end_date=${formatDate(endDate)}&depth=${depth}&category=${category.category_title}`;

	plotLink.addEventListener("click", (event) => {
		event.stopPropagation();
	});

	summary.appendChild(nameSpan);
	summary.appendChild(deltaDiv);
	summary.appendChild(clicksDiv);
	summary.appendChild(plotLink);
	details.appendChild(summary);
	details.appendChild(impressionsDiv);
	details.addEventListener("toggle", handleAccordionToggle);

	return details;
}

async function handleAccordionToggle(event) {
	const details = event.target;
	if (!details.open || details.dataset.loaded) {
		return;
	}

	const categoryQid = details.dataset.categoryQid;
	const categoryTitle = details.dataset.categoryTitle;

	const articlesContainer = document.createElement("div");
	articlesContainer.className = "articles-container";
	articlesContainer.innerHTML =
		'<div class="loading-indicator">Loading articles</div>';
	details.appendChild(articlesContainer);

	try {
		const wiki = document.getElementById("wiki").value;
		const baselineStartDate = document.getElementById(
			"baseline_start_date",
		).value;
		const baselineEndDate = document.getElementById("baseline_end_date").value;
		const impactStartDate = document.getElementById("impact_start_date").value;
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
			100,
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

function renderArticles(container, articles, wiki) {
	container.innerHTML = "";
	for (const article of articles) {
		const row = document.createElement("div");
		row.className = `article-item ${article.delta_percentage >= 0 ? "positive" : "negative"}`;

		const title = document.createElement("a");
		title.className = "article-title";
		title.textContent = article.article_title.replace(/_/g, " ");
		title.href = `https://${wiki.replace("wiki", "")}.wikipedia.org/wiki/${encodeURIComponent(article.article_title)}`;
		title.target = "_blank";
		title.rel = "noopener noreferrer";

		const delta = document.createElement("span");
		delta.className = `article-delta ${article.delta_percentage >= 0 ? "positive" : "negative"}`;
		delta.textContent = `${article.delta_percentage >= 0 ? "+" : ""}${article.delta_percentage.toFixed(2)}%`;

		const metrics = document.createElement("div");
		metrics.className = "article-views";
		metrics.textContent = `Clicks: ${article.baseline_clicks.toLocaleString()} → ${article.impact_clicks.toLocaleString()} | Impr: ${article.baseline_impressions.toLocaleString()} → ${article.impact_impressions.toLocaleString()}`;

		const plotLink = document.createElement("a");
		plotLink.className = "plot-button";
		plotLink.textContent = "📊";
		plotLink.title = "View trend chart";
		plotLink.target = "_blank";
		plotLink.rel = "noopener noreferrer";

		const endDate = new Date();
		endDate.setDate(endDate.getDate() - 1);
		const startDate = new Date();
		startDate.setMonth(startDate.getMonth() - 1);
		const formatDate = (date) => date.toISOString().split("T")[0];
		plotLink.href = `/googlesearch/trends?type=article&wiki=${wiki}&start_date=${formatDate(startDate)}&end_date=${formatDate(endDate)}&article=${article.article_title}`;

		row.appendChild(title);
		row.appendChild(delta);
		row.appendChild(metrics);
		row.appendChild(plotLink);
		container.appendChild(row);
	}
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

	if (wiki) {
		document.getElementById("wiki").value = wiki;
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
	if (depth) {
		document.getElementById("depth").value = depth;
	}
	if (limit) {
		document.getElementById("limit").value = limit;
	}

	if (wiki) {
		onSubmit(new Event("submit"));
	} else {
		document.querySelector(".examples").hidden = false;
		const now = new Date();
		const oneMonthAgo = new Date(
			now.getFullYear(),
			now.getMonth() - 1,
			now.getDate(),
		);
		const twoMonthsAgo = new Date(
			now.getFullYear(),
			now.getMonth() - 2,
			now.getDate(),
		);
		document.getElementById("impact_end_date").value = now
			.toISOString()
			.slice(0, 10);
		document.getElementById("baseline_start_date").value = twoMonthsAgo
			.toISOString()
			.slice(0, 10);
		document.getElementById("baseline_end_date").value = oneMonthAgo
			.toISOString()
			.slice(0, 10);
		document.getElementById("impact_start_date").value = oneMonthAgo
			.toISOString()
			.slice(0, 10);
	}
}
