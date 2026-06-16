import {
	renderCategoryChips,
	searchCategories,
} from "./utils/category-chips.js";
import { initializeChart, updateChart } from "./utils/chart-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { renderPageviewsTopArticles } from "./utils/top-articles-table.js";
import { showMessage } from "./utils/ui-utils.js";
import "./components/wiki-selector.js";

document.addEventListener("DOMContentLoaded", async () => {
	document.getElementById("trend-form").addEventListener("submit", onSubmit);

	// Set up wiki selector change handler
	const wikiSelector = document.getElementById("wiki");
	const articleElement = document.getElementById("article");
	const categoryElement = document.getElementById("category");

	wikiSelector.addEventListener("change", function () {
		const wikiValue = this.value.replaceAll("wiki", "");
		articleElement?.setAttribute("wiki", wikiValue);
		categoryElement?.setAttribute("wiki", wikiValue);
	});

	// Initialize with current wiki value
	const wikiValue = wikiSelector.value.replaceAll("wiki", "");

	articleElement.setAttribute("wiki", wikiValue);
	categoryElement.setAttribute("wiki", wikiValue);

	document
		.getElementById("trend-form")
		.addEventListener("form-fill-complete", () => {
			onSubmit(new Event("submit"));
		});

});
function showSection(section) {
	const chart = document.getElementById("chart");
	const topArticles = document.getElementById("top-articles");
	const categoryList = document.getElementById("category-list");
	const wikiTrends = document.querySelector("wiki-trends");

	// Clear/hide all sections
	if (chart) chart.style.display = "none";
	if (topArticles) topArticles.innerHTML = "";
	if (wikiTrends) wikiTrends.remove();

	// Show requested section
	if (section === "chart") {
		if (chart) chart.style.display = "block";
	} else if (section === "chart-with-articles") {
		if (chart) chart.style.display = "block";
		// top-articles will be populated separately
	} else if (section === "wiki-trends") {
		// wiki-trends component will be added separately
		// Also clear category list when showing wiki-trends
		if (categoryList) categoryList.innerHTML = "";
	}
}

async function onSubmit(event) {
	event.preventDefault();

	document.querySelector(".examples").hidden = true;

	const params = new URLSearchParams();
	const type = document.querySelector('input[name="type"]:checked').value;
	const wiki = document.getElementById("wiki").value;
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	const category_qid = document.getElementById("category_qid").value;
	const article_qid = document.getElementById("article_qid").value;

	params.append("type", type);
	params.append("wiki", wiki);
	params.append("start_date", startDate);
	params.append("end_date", endDate);
	if (category_qid) {
		params.append("category_qid", category_qid);
	}
	if (article_qid) {
		params.append("article_qid", article_qid);
	}
	try {
		if (type === "topic") {
			const topic = document.getElementById("topic").value.replaceAll(" ", "_");
			params.append("topic", topic);

			const newUrl = `${window.location.pathname}?${params.toString()}`;
			window.history.pushState({}, "", newUrl);

			await searchTopicCategories(wiki, topic);
		} else if (type === "category") {
			const category = document
				.getElementById("category")
				.value.replaceAll(" ", "_");
			params.append("category", category);

			// Update the browser URL with the new parameters
			const newUrl = `${window.location.pathname}?${params.toString()}`;
			window.history.pushState({}, "", newUrl);

			await fetchCategoryPageviews(wiki, category, startDate, endDate);
			await renderSubCategories(wiki, category);
		} else if (type === "article") {
			const article = document
				.getElementById("article")
				.value.replaceAll(" ", "_");
			params.append("article", article);

			// Update the browser URL with the new parameters
			const newUrl = `${window.location.pathname}?${params.toString()}`;
			window.history.pushState({}, "", newUrl);
			await fetchArticlePageviews(wiki, article, startDate, endDate);
		}
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch data. Please try again.", "error");
	}
}

let chartInstance = null;

function ensureChartInitialized() {
	if (!chartInstance) {
		const chartElement = document.getElementById("chart");
		chartInstance = initializeChart(chartElement, "Pageviews Trend");
	}
}

function updateChartWithData(data, label) {
	ensureChartInitialized();
	updateChart(chartInstance, data, label);
}

function plotCategory(wiki, qid, title) {
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	fetchCategoryPageviews(wiki, title, startDate, endDate, qid);
}

async function searchTopicCategories(wiki, topic) {
	// No chart until the user picks one of the matched categories.
	document.getElementById("chart").style.display = "none";
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

async function renderSubCategories(wiki, category) {
	const categoryListContainer = document.getElementById("category-list");
	const apiUrl = `/api/list/sub_categories?wiki=${wiki}&category=${category}`;

	showProgress();
	try {
		const response = await fetch(apiUrl);
		if (!response.ok) {
			throw new Error("Failed to fetch data");
		}
		const subcategories = await response.json();
		renderCategoryChips(categoryListContainer, {
			heading: "Subcategories",
			items: subcategories,
			wiki,
			onPlot: (qid, title) => plotCategory(wiki, qid, title),
		});
	} finally {
		hideProgress();
	}
}

function renderTopArticles(wiki, topArticles) {
	const container = document.getElementById("top-articles");
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	renderPageviewsTopArticles(container, wiki, topArticles, startDate, endDate);
}

document.addEventListener("DOMContentLoaded", () => {
	const startDatePicker = document.getElementById("start_date");
	const endDatePicker = document.getElementById("end_date");
	const today = new Date();

	// Format the date to "YYYY-MM-DD" as required by the input type="date"
	let year = today.getFullYear();
	let month = String(today.getMonth() + 1).padStart(2, "0");
	let day = String(today.getDate()).padStart(2, "0");
	endDatePicker.value = `${year}-${month}-${day}`;

	const oneMonthAgo = new Date(
		today.getFullYear(),
		today.getMonth() - 1,
		today.getDate(),
	);
	year = oneMonthAgo.getFullYear();
	month = String(oneMonthAgo.getMonth() + 1).padStart(2, "0");
	day = String(oneMonthAgo.getDate()).padStart(2, "0");

	startDatePicker.value = `${year}-${month}-${day}`;
});

async function fetchCategoryPageviews(
	wiki,
	category,
	startDate,
	endDate,
	categoryQid,
) {
	showSection("chart-with-articles");

	let apiUrl = `/api/pageviews/category?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&category=${encodeURIComponent(
		category,
	)}`;
	if (categoryQid) {
		apiUrl += `&category_qid=${categoryQid}`;
	}
	const label = `Category: ${wiki} - ${category.replaceAll("_", " ")}`;

	try {
		const startTime = performance.now();
		showProgress();
		const response = await fetch(apiUrl);
		if (!response.ok) {
			throw new Error("Failed to fetch data");
		}

		const data = await response.json();
		hideProgress();
		updateChartWithData(data.views, label);
		const endTime = performance.now();
		const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
		showMessage(`Fetched ${label} in ${timeTaken} seconds.`, "success");

		if (data.top_articles && data.top_articles.length > 0) {
			renderTopArticles(wiki, data.top_articles);
		}
	} catch (error) {
		hideProgress();
		console.error("Error:", error);
		showMessage("Failed to fetch category data. Please try again.", "error");
	}
}

async function fetchArticlePageviews(wiki, article, startDate, endDate) {
	showSection("chart");

	const apiUrl = `/api/pageviews/article?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&article=${encodeURIComponent(
		article,
	)}`;
	const label = `Article: ${wiki} - ${article.replaceAll("_", " ")}`;

	try {
		const startTime = performance.now();
		showProgress();
		const response = await fetch(apiUrl);
		if (!response.ok) {
			throw new Error("Failed to fetch data");
		}

		const data = await response.json();
		hideProgress();
		updateChartWithData(data.views, label);
		const endTime = performance.now();
		const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
		showMessage(`Fetched ${label} in ${timeTaken} seconds.`, "success");
	} catch (error) {
		hideProgress();
		console.error("Error:", error);
		showMessage("Failed to fetch article data. Please try again.", "error");
	}
}
