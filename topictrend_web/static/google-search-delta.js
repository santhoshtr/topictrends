import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import "./components/wiki-selector.js";

document.addEventListener("DOMContentLoaded", () => {
	document.getElementById("delta-form").addEventListener("submit", onSubmit);

	document.getElementById("delta-form").addEventListener("form-fill-complete", () => {
		onSubmit(new Event("submit"));
	});

	if (!window.location.search) {
		document.querySelector(".examples").hidden = false;
		const now = new Date();
		const oneMonthAgo = new Date(now.getFullYear(), now.getMonth() - 1, now.getDate());
		const twoMonthsAgo = new Date(now.getFullYear(), now.getMonth() - 2, now.getDate());
		document.getElementById("impact_end_date").value = now.toISOString().slice(0, 10);
		document.getElementById("baseline_start_date").value = twoMonthsAgo.toISOString().slice(0, 10);
		document.getElementById("baseline_end_date").value = oneMonthAgo.toISOString().slice(0, 10);
		document.getElementById("impact_start_date").value = oneMonthAgo.toISOString().slice(0, 10);
	}
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
	details.dataset.baselineClicks = category.baseline_clicks;
	details.dataset.impactClicks = category.impact_clicks;
	details.dataset.baselineImpressions = category.baseline_impressions;
	details.dataset.impactImpressions = category.impact_impressions;

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

	const plotLink = document.createElement("a");
	plotLink.className = "plot-button";
	plotLink.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
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
	summary.appendChild(plotLink);
	details.appendChild(summary);
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
			renderArticles(articlesContainer, articlesData.articles, wiki, {
				baselineClicks: Number.parseInt(
					details.dataset.baselineClicks || "0",
					10,
				),
				impactClicks: Number.parseInt(details.dataset.impactClicks || "0", 10),
				baselineImpressions: Number.parseInt(
					details.dataset.baselineImpressions || "0",
					10,
				),
				impactImpressions: Number.parseInt(
					details.dataset.impactImpressions || "0",
					10,
				),
			});
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

function renderArticles(container, articles, wiki, summaryMetrics) {
	container.innerHTML = "";

	const impressionsDeltaPercent =
		summaryMetrics.baselineImpressions === 0
			? summaryMetrics.impactImpressions > 0
				? 100
				: 0
			: ((summaryMetrics.impactImpressions -
					summaryMetrics.baselineImpressions) /
					summaryMetrics.baselineImpressions) *
				100;

	const summary = document.createElement("div");
	summary.className = "articles-summary";

	const clicksRow = document.createElement("div");
	clicksRow.className = "articles-summary-row";
	clicksRow.textContent = `Clicks: ${summaryMetrics.baselineClicks.toLocaleString()}  →  ${summaryMetrics.impactClicks.toLocaleString()}`;

	const impressionsRow = document.createElement("div");
	impressionsRow.className = "articles-summary-row";
	const sign = impressionsDeltaPercent >= 0 ? "+" : "";
	impressionsRow.textContent = `Impressions: ${summaryMetrics.baselineImpressions.toLocaleString()}  →  ${summaryMetrics.impactImpressions.toLocaleString()} (${sign}${impressionsDeltaPercent.toFixed(2)}%)`;

	summary.appendChild(clicksRow);
	summary.appendChild(impressionsRow);
	container.appendChild(summary);

	const table = document.createElement("table");
	table.className = "gs-delta-articles-table";

	const thead = document.createElement("thead");
	const headerRow = document.createElement("tr");
	for (const label of ["Article", "Clicks", "Impressions", "Delta", "Plot"]) {
		const th = document.createElement("th");
		th.textContent = label;
		headerRow.appendChild(th);
	}
	thead.appendChild(headerRow);
	table.appendChild(thead);

	const tbody = document.createElement("tbody");
	for (const article of articles) {
		const row = document.createElement("tr");

		const titleCell = document.createElement("td");
		const titleLink = document.createElement("a");
		titleLink.className = "article-title";
		titleLink.textContent = article.article_title.replace(/_/g, " ");
		titleLink.href = `https://${wiki.replace("wiki", "")}.wikipedia.org/wiki/${encodeURIComponent(article.article_title)}`;
		titleLink.target = "_blank";
		titleLink.rel = "noopener noreferrer";
		titleCell.appendChild(titleLink);

		const clicksCell = document.createElement("td");
		clicksCell.textContent = `${article.baseline_clicks.toLocaleString()} → ${article.impact_clicks.toLocaleString()}`;

		const impressionsCell = document.createElement("td");
		impressionsCell.textContent = `${article.baseline_impressions.toLocaleString()}  → ${article.impact_impressions.toLocaleString()}`;

		const deltaCell = document.createElement("td");
		deltaCell.className =
			article.delta_percentage >= 0
				? "article-delta positive"
				: "article-delta negative";
		deltaCell.textContent = `${article.delta_percentage >= 0 ? "+" : ""}${article.delta_percentage.toFixed(2)}%`;

		const plotCell = document.createElement("td");
		const plotLink = document.createElement("a");
		plotLink.className = "plot-button";
		plotLink.innerHTML =
			'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
		plotLink.title = "View trend chart";
		plotLink.target = "_blank";
		plotLink.rel = "noopener noreferrer";
		plotLink.href = `/googlesearch/trends?type=article&wiki=${wiki}&article=${encodeURIComponent(article.article_title)}`;
		plotCell.appendChild(plotLink);

		row.appendChild(titleCell);
		row.appendChild(clicksCell);
		row.appendChild(impressionsCell);
		row.appendChild(deltaCell);
		row.appendChild(plotCell);
		tbody.appendChild(row);
	}

	table.appendChild(tbody);
	container.appendChild(table);
}
