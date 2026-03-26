import { autocomp } from "./autocomp.js";
import { initializeChart, updateChart } from "./utils/chart-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { renderPageviewsTopArticles } from "./utils/top-articles-table.js";
import { showMessage } from "./utils/ui-utils.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

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

	await populateWikiDropdown();
	populateFormFromQueryParams();
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
	const depth = document.getElementById("depth").value;

	params.append("type", type);
	params.append("wiki", wiki);
	params.append("start_date", startDate);
	params.append("end_date", endDate);
	params.append("depth", depth);
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

			await fetchTopicPageviews(wiki, topic, startDate, endDate, 0);
		} else if (type === "category") {
			const category = document
				.getElementById("category")
				.value.replaceAll(" ", "_");
			params.append("category", category);

			// Update the browser URL with the new parameters
			const newUrl = `${window.location.pathname}?${params.toString()}`;
			window.history.pushState({}, "", newUrl);

			await fetchCategoryPageviews(wiki, category, startDate, endDate, depth);
			await renderSubCategories(wiki, category, depth);
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

async function renderSubCategories(wiki, category, depth = 4) {
	const categoryListContainer = document.getElementById("category-list");
	const apiUrl = `https://topictrends.wmcloud.org/api/list/sub_categories?wiki=${wiki}&category=${category}`;

	showProgress();
	const response = await fetch(apiUrl);
	const subcategories = await response.json();
	hideProgress();
	if (!response.ok) {
		throw new Error("Failed to fetch data");
	}

	categoryListContainer.innerHTML = "";

	const subheading = document.createElement("h3");
	subheading.textContent = "Subcategories";
	categoryListContainer.appendChild(subheading);

	const ul = document.createElement("ul");
	Object.entries(subcategories).forEach(([qid, title]) => {
		const li = document.createElement("li");
		li.id = qid;

		const wikiCategory = document.createElement("wiki-category");
		wikiCategory.setAttribute("title", title);
		wikiCategory.setAttribute("qid", qid);
		wikiCategory.setAttribute("views", "0");

		const plotButton = document.createElement("button");
		plotButton.title = "Plot pageviews for this category";
		plotButton.className = "plot-button";
		plotButton.innerHTML = `
      <svg xmlns="http://www.w3.org/2000/svg" 
        height="16px" viewBox="0 -960 960 960"
        width="16px" fill="currentColor">
      <path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/>
      </svg>
      `;
		plotButton.addEventListener("click", (event) => {
			event.preventDefault();
			const startDate = document.getElementById("start_date").value;
			const endDate = document.getElementById("end_date").value;

			fetchCategoryPageviews(wiki, title, startDate, endDate, depth);
		});

		li.appendChild(wikiCategory);
		li.appendChild(plotButton);
		ul.appendChild(li);
	});

	categoryListContainer.appendChild(ul);
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

async function fetchTopicPageviews(wiki, topic, startDate, endDate, depth) {
	showSection("chart-with-articles");

	const apiUrl = `https://topictrends.wmcloud.org/api/pageviews/topic?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&depth=${depth}&topic=${encodeURIComponent(
		topic,
	)}`;
	const label = `Topic: ${wiki} - ${topic.replaceAll("_", " ")}`;

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
		showMessage("Failed to fetch topic data. Please try again.", "error");
	}
}

async function fetchCategoryPageviews(
	wiki,
	category,
	startDate,
	endDate,
	depth,
) {
	showSection("chart-with-articles");

	const apiUrl = `https://topictrends.wmcloud.org/api/pageviews/category?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&depth=${depth}&category=${encodeURIComponent(
		category,
	)}`;
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

	const apiUrl = `https://topictrends.wmcloud.org/api/pageviews/article?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&article=${encodeURIComponent(
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

function populateFormFromQueryParams() {
	const urlParams = new URLSearchParams(window.location.search);

	const type = urlParams.get("type");
	const wiki = urlParams.get("wiki");
	const startDate = urlParams.get("start_date");
	const endDate = urlParams.get("end_date");
	const topic = urlParams.get("topic");
	const category = urlParams.get("category");
	const category_qid = urlParams.get("category_qid");
	const article = urlParams.get("article");
	const article_qid = urlParams.get("article_qid");
	const depth = urlParams.get("depth");

	if (type) {
		document.querySelector(`input[name="type"][value="${type}"]`).checked =
			true;
	}
	if (depth) {
		document.getElementById("depth").value = depth;
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
	if (type === "topic" && topic) {
		document.getElementById("topic").value = topic.replaceAll("_", " ");
	}
	if (type === "category" && category) {
		document.getElementById("category").value = category.replaceAll("_", " ");
		if (category_qid) {
			document.getElementById("category_qid").value = category_qid;
		}
	}
	if (type === "article" && article) {
		document.getElementById("article").value = article.replaceAll("_", " ");
		if (article_qid) {
			document.getElementById("article_qid").value = article_qid;
		}
	}

	if (type && wiki && startDate && endDate) {
		onSubmit(new Event("submit"));
	} else {
		document.querySelector(".examples").hidden = false;
	}
}

document.addEventListener("DOMContentLoaded", () => {
	const loadButton = document.getElementById("wikitrends-btn");

	loadButton?.addEventListener("click", () => {
		showSection("wiki-trends");

		let topicTrends = document.querySelector("wiki-trends");
		const selectedWiki = document.getElementById("wiki").value;
		const startDate = document.getElementById("start_date").value;
		const endDate = document.getElementById("end_date").value;

		if (!topicTrends) {
			const topicTrendsEl = document.createElement("wiki-trends");
			document.querySelector(".main").appendChild(topicTrendsEl);
			topicTrends = document.querySelector("wiki-trends");
		}

		topicTrends.setAttribute("wiki", selectedWiki);
		topicTrends.setAttribute("start_date", startDate);
		topicTrends.setAttribute("end_date", endDate);
		loadButton.disabled = true;

		setTimeout(() => {
			loadButton.disabled = false;
		}, 1000);
	});
});
