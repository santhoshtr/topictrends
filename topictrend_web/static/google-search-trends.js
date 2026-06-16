import {
	renderCategoryChips,
	searchCategories,
} from "./utils/category-chips.js";
import { initializeChart } from "./utils/chart-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { renderGoogleSearchTopArticles } from "./utils/top-articles-table.js";
import { showMessage } from "./utils/ui-utils.js";
import "./components/wiki-selector.js";

document.addEventListener("DOMContentLoaded", () => {
	document.getElementById("trend-form").addEventListener("submit", onSubmit);

	const wikiSelector = document.getElementById("wiki");
	const articleElement = document.getElementById("article");
	const categoryElement = document.getElementById("category");

	wikiSelector.addEventListener("change", function () {
		const wikiValue = this.value.replaceAll("wiki", "");
		articleElement?.setAttribute("wiki", wikiValue);
		categoryElement?.setAttribute("wiki", wikiValue);
	});

	document
		.getElementById("trend-form")
		.addEventListener("form-fill-complete", () => {
			onSubmit(new Event("submit"));
		});

	if (!window.location.search) {
		const now = new Date();
		const monthAgo = new Date(
			now.getFullYear(),
			now.getMonth() - 1,
			now.getDate(),
		);
		document.getElementById("end_date").value = now.toISOString().slice(0, 10);
		document.getElementById("start_date").value = monthAgo
			.toISOString()
			.slice(0, 10);
	}
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
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	renderGoogleSearchTopArticles(
		container,
		wiki,
		topArticles,
		startDate,
		endDate,
	);
}

async function onSubmit(event) {
	event.preventDefault();
	document.querySelector(".examples").hidden = true;

	const params = new URLSearchParams();
	const type = document.querySelector('input[name="type"]:checked').value;
	const wiki = document.getElementById("wiki").value;
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;

	params.append("type", type);
	params.append("wiki", wiki);
	params.append("start_date", startDate);
	params.append("end_date", endDate);

	try {
		if (type === "topic") {
			const topic = document.getElementById("topic").value.replaceAll(" ", "_");
			params.append("topic", topic);
			window.history.pushState(
				{},
				"",
				`${window.location.pathname}?${params.toString()}`,
			);
			await searchTopicCategories(wiki, topic);
		} else if (type === "category") {
			const category = document
				.getElementById("category")
				.value.replaceAll(" ", "_");
			params.append("category", category);
			window.history.pushState(
				{},
				"",
				`${window.location.pathname}?${params.toString()}`,
			);
			await fetchCategorySearchData(wiki, category, startDate, endDate);
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

function plotCategory(wiki, qid, title) {
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	fetchCategorySearchData(wiki, title, startDate, endDate, qid);
}

async function searchTopicCategories(wiki, topic) {
	// No chart until the user picks one of the matched categories.
	document.getElementById("chart").style.display = "none";
	document.getElementById("ctr-summary").innerHTML = "";
	document.getElementById("top-articles").innerHTML = "";

	showProgress();
	try {
		const items = await searchCategories(wiki, topic);
		renderCategoryChips(document.getElementById("category-list"), {
			heading: "Matched categories",
			items,
			wiki,
			onPlot: (qid, title) => plotCategory(wiki, qid, title),
		});
		if (items.length === 0) {
			showMessage(
				"No matching categories found. Try a different topic.",
				"error",
			);
		}
	} finally {
		hideProgress();
	}
}

async function fetchCategorySearchData(
	wiki,
	category,
	startDate,
	endDate,
	categoryQid,
) {
	let url = `/api/googlesearch/category?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&category=${encodeURIComponent(category)}`;
	if (categoryQid) {
		url += `&category_qid=${categoryQid}`;
	}
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
	const url = `/api/googlesearch/article?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&article=${encodeURIComponent(article)}`;
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
