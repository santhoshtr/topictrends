import { initializeChart } from "./utils/chart-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

document.addEventListener("DOMContentLoaded", async () => {
	document.getElementById("trend-form").addEventListener("submit", onSubmit);

	const wikiSelector = document.getElementById("wiki");
	const articleElement = document.getElementById("article");
	const categoryElement = document.getElementById("category");

	wikiSelector.addEventListener("change", function () {
		const wikiValue = this.value.replaceAll("wiki", "");
		articleElement?.setAttribute("wiki", wikiValue);
		categoryElement?.setAttribute("wiki", wikiValue);
	});

	await populateWikiDropdown();

	const wikiValue = wikiSelector.value.replaceAll("wiki", "");
	articleElement?.setAttribute("wiki", wikiValue);
	categoryElement?.setAttribute("wiki", wikiValue);

	populateFormFromQueryParams();
});

let chartInstance = null;

function ensureChartInitialized() {
	if (!chartInstance) {
		const chartElement = document.getElementById("chart");
		chartInstance = initializeChart(chartElement, "Google Search Trend");
	}
}

function updateChartWithSearchData(data, label) {
	ensureChartInitialized();
	document.getElementById("chart").style.display = "block";

	const dates = data.map((item) => item.date);
	const clicks = data.map((item) => item.clicks);
	const impressions = data.map((item) => item.impressions);

	chartInstance.setOption({
		title: { text: "Google Search Trend" },
		xAxis: { data: dates },
		series: [
			{
				name: `${label} - Clicks`,
				type: "line",
				smooth: true,
				data: clicks,
			},
			{
				name: `${label} - Impressions`,
				type: "line",
				smooth: true,
				data: impressions,
			},
		],
		legend: { top: "bottom", left: "center" },
	});
}

function renderCtrSummary(title, search) {
	const ctrSummary = document.getElementById("ctr-summary");
	if (!search || search.length === 0) {
		ctrSummary.innerHTML = "";
		return;
	}

	const totalClicks = search.reduce((sum, row) => sum + row.clicks, 0);
	const totalImpressions = search.reduce(
		(sum, row) => sum + row.impressions,
		0,
	);
	const ctr = totalImpressions > 0 ? totalClicks / totalImpressions : 0;

	ctrSummary.innerHTML = "";
	const heading = document.createElement("h3");
	heading.textContent = `CTR Summary: ${title.replaceAll("_", " ")}`;
	ctrSummary.appendChild(heading);

	const cards = document.createElement("div");
	cards.className = "gs-metric-cards";

	const metrics = [
		{ title: "Clicks", value: totalClicks.toLocaleString() },
		{ title: "Impressions", value: totalImpressions.toLocaleString() },
		{ title: "CTR", value: `${(ctr * 100).toFixed(2)}%` },
	];

	for (const metric of metrics) {
		const card = document.createElement("div");
		card.className = "gs-metric-card";

		const metricTitle = document.createElement("div");
		metricTitle.className = "gs-metric-title";
		metricTitle.textContent = metric.title;

		const metricValue = document.createElement("div");
		metricValue.className = "gs-metric-value";
		metricValue.textContent = metric.value;

		card.appendChild(metricTitle);
		card.appendChild(metricValue);
		cards.appendChild(card);
	}

	ctrSummary.appendChild(cards);
}

function renderTopArticles(wiki, topArticles) {
	const container = document.getElementById("top-articles");
	container.innerHTML = "";

	if (!topArticles || topArticles.length === 0) {
		return;
	}

	const heading = document.createElement("h3");
	heading.textContent = "Top Articles in Category";
	container.appendChild(heading);

	const table = document.createElement("table");
	table.className = "gs-top-articles-table";

	const thead = document.createElement("thead");
	const headerRow = document.createElement("tr");
	for (const label of ["Article", "Clicks", "Impressions", "CTR", "Plot"]) {
		const th = document.createElement("th");
		th.textContent = label;
		headerRow.appendChild(th);
	}
	thead.appendChild(headerRow);
	table.appendChild(thead);

	const tbody = document.createElement("tbody");
	for (const article of topArticles) {
		const row = document.createElement("tr");

		const articleCell = document.createElement("td");
		const articleLink = document.createElement("a");
		articleLink.className = "article-title";
		articleLink.textContent = article.title.replaceAll("_", " ");
		articleLink.href = `https://${wiki.replace("wiki", "")}.wikipedia.org/wiki/${encodeURIComponent(article.title)}`;
		articleLink.target = "_blank";
		articleLink.rel = "noopener noreferrer";
		articleCell.appendChild(articleLink);

		const clicksCell = document.createElement("td");
		clicksCell.textContent = article.clicks.toLocaleString();

		const impressionsCell = document.createElement("td");
		impressionsCell.textContent = article.impressions.toLocaleString();

		const ctrCell = document.createElement("td");
		ctrCell.textContent = `${(article.ctr * 100).toFixed(2)}%`;

		const plotCell = document.createElement("td");
		const plotLink = document.createElement("a");
		plotLink.href = `/googlesearch/trends?type=article&wiki=${wiki}&article=${encodeURIComponent(article.title)}`;
		plotLink.textContent =
			'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
		plotLink.title = "Plot article trend";
		plotCell.appendChild(plotLink);

		row.appendChild(articleCell);
		row.appendChild(clicksCell);
		row.appendChild(impressionsCell);
		row.appendChild(ctrCell);
		row.appendChild(plotCell);
		tbody.appendChild(row);
	}
	table.appendChild(tbody);
	container.appendChild(table);
}

async function onSubmit(event) {
	event.preventDefault();
	document.querySelector(".examples").hidden = true;

	const params = new URLSearchParams();
	const type = document.querySelector('input[name="type"]:checked').value;
	const wiki = document.getElementById("wiki").value;
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	const depth = document.getElementById("depth").value;

	params.append("type", type);
	params.append("wiki", wiki);
	params.append("start_date", startDate);
	params.append("end_date", endDate);
	params.append("depth", depth);

	try {
		if (type === "category") {
			const category = document
				.getElementById("category")
				.value.replaceAll(" ", "_");
			params.append("category", category);
			window.history.pushState(
				{},
				"",
				`${window.location.pathname}?${params.toString()}`,
			);
			await fetchCategorySearchData(wiki, category, startDate, endDate, depth);
		} else {
			const article = document
				.getElementById("article")
				.value.replaceAll(" ", "_");
			params.append("article", article);
			window.history.pushState(
				{},
				"",
				`${window.location.pathname}?${params.toString()}`,
			);
			await fetchArticleSearchData(wiki, article, startDate, endDate);
		}
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch data. Please try again.", "error");
	}
}

async function fetchCategorySearchData(
	wiki,
	category,
	startDate,
	endDate,
	depth,
) {
	const url = `https://topictrends.wmcloud.org/api/googlesearch/category?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&depth=${depth}&category=${encodeURIComponent(category)}`;
	const label = `Category: ${wiki} - ${category.replaceAll("_", " ")}`;

	try {
		showProgress();
		const startTime = performance.now();
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error("Failed to fetch category search data");
		}

		const data = await response.json();
		updateChartWithSearchData(data.search, label);
		renderCtrSummary(data.title, data.search);
		renderTopArticles(wiki, data.top_articles);

		const elapsed = ((performance.now() - startTime) / 1000).toFixed(2);
		showMessage(`Fetched ${label} in ${elapsed} seconds.`, "success");
	} finally {
		hideProgress();
	}
}

async function fetchArticleSearchData(wiki, article, startDate, endDate) {
	const url = `https://topictrends.wmcloud.org/api/googlesearch/article?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&article=${encodeURIComponent(article)}`;
	const label = `Article: ${wiki} - ${article.replaceAll("_", " ")}`;

	try {
		showProgress();
		const startTime = performance.now();
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error("Failed to fetch article search data");
		}

		const data = await response.json();
		updateChartWithSearchData(data.search, label);
		renderCtrSummary(data.title, data.search);
		renderTopArticles(wiki, []);

		const elapsed = ((performance.now() - startTime) / 1000).toFixed(2);
		showMessage(`Fetched ${label} in ${elapsed} seconds.`, "success");
	} finally {
		hideProgress();
	}
}

function populateFormFromQueryParams() {
	const urlParams = new URLSearchParams(window.location.search);
	const type = urlParams.get("type");
	const wiki = urlParams.get("wiki");
	const startDate = urlParams.get("start_date");
	const endDate = urlParams.get("end_date");
	const category = urlParams.get("category");
	const article = urlParams.get("article");
	const depth = urlParams.get("depth");

	if (type) {
		document.querySelector(`input[name="type"][value="${type}"]`).checked =
			true;
	}
	if (wiki) {
		document.getElementById("wiki").value = wiki;
	}
	if (startDate) {
		document.getElementById("start_date").value = startDate;
	}
	if (endDate) {
		document.getElementById("end_date").value = endDate;
	}
	if (depth) {
		document.getElementById("depth").value = depth;
	}
	if (type === "category" && category) {
		document.getElementById("category").value = category.replaceAll("_", " ");
	}
	if (type === "article" && article) {
		document.getElementById("article").value = article.replaceAll("_", " ");
	}

	if (type && wiki && startDate && endDate) {
		onSubmit(new Event("submit"));
	} else {
		document.querySelector(".examples").hidden = false;
		const endDateInput = document.getElementById("end_date");
		const startDateInput = document.getElementById("start_date");
		const now = new Date();
		const monthAgo = new Date(
			now.getFullYear(),
			now.getMonth() - 1,
			now.getDate(),
		);
		endDateInput.value = now.toISOString().slice(0, 10);
		startDateInput.value = monthAgo.toISOString().slice(0, 10);
	}
}
