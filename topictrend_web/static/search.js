import { autocomp } from "./autocomp.js";

document.addEventListener("DOMContentLoaded", async function () {
	document.getElementById("search-form").addEventListener("submit", onSubmit);

	// Set up wiki selector change handler
	const wikiSelector = document.getElementById("wiki");
	const categoryElement = document.getElementById("category");

	// Initialize with current wiki value
	const wikiValue = wikiSelector.value.replaceAll("wiki", "");

	categoryElement.setAttribute("wiki", wikiValue);

	await populateWikiDropdown();
	populateFormFromQueryParams();
	document
		.getElementById("categories-trends-btn")
		.addEventListener("click", onTrendBtnClick);
});

async function onSubmit(event) {
	event.preventDefault();

	const params = new URLSearchParams();
	const wiki = document.getElementById("wiki").value;
	const match_threshold = document.getElementById("match_threshold").value;

	params.append("wiki", wiki);
	try {
		const category = document
			.getElementById("category")
			.value.replaceAll(" ", "_");
		params.append("category", category);
		params.append("match_threshold", match_threshold);
		// Update the browser URL with the new parameters
		const newUrl = `${window.location.pathname}?${params.toString()}`;
		window.history.pushState({}, "", newUrl);

		const categories = await searchCategory(wiki, category, match_threshold);
		renderCategories(categories, wiki);
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch data. Please try again.", "error");
	}
}

async function onTrendBtnClick(event) {
	event.preventDefault();

	const params = new URLSearchParams();
	const wiki = document.getElementById("wiki").value;
	const match_threshold = document.getElementById("match_threshold").value;
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;
	const depth = 1;
	params.append("wiki", wiki);
	try {
		const category = document
			.getElementById("category")
			.value.replaceAll(" ", "_");
		params.append("category", category);
		params.append("match_threshold", match_threshold);
		// Update the browser URL with the new parameters
		const newUrl = `${window.location.pathname}?${params.toString()}`;
		window.history.pushState({}, "", newUrl);

		const categories = await fetchCategoryPageviews(
			wiki,
			category,
			match_threshold,
			startDate,
			endDate,
			depth,
		);
		renderCategories(categories, wiki);
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch data. Please try again.", "error");
	}
}

async function fetchCategoryPageviews(
	wiki,
	category,
	match_threshold,
	startDate,
	endDate,
	depth,
) {
	const apiUrl = `/api/pageviews/categories?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&depth=${depth}&category_query=${encodeURIComponent(
		category,
	)}&match_threshold=${match_threshold}`;
	const label = `Category: ${wiki} - ${category.replaceAll("_", " ")}`;

	try {
		const startTime = performance.now();
		const response = await fetch(apiUrl);
		if (!response.ok) {
			throw new Error("Failed to fetch data");
		}

		const data = await response.json();
		updateChart(data.cumulative_views, label);
		const endTime = performance.now();
		const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
		showMessage(`Fetched ${label} in ${timeTaken} seconds.`, "success");

		if (data.top_articles && data.top_articles.length > 0) {
			renderTopArticles(wiki, data.top_articles);
		}
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch category data. Please try again.", "error");
	}
}

let chartInstance = null;

function initializeChart() {
	const theme = window.matchMedia("(prefers-color-scheme: dark)").matches
		? "dark"
		: "light";
	const chartElement = document.getElementById("chart");
	chartInstance = echarts.init(chartElement, theme, {
		renderer: "svg",
	});

	const initialOption = {
		darkMode: "auto",
		color: [
			"#4b77d6",
			"#eeb533",
			"#fd7865",
			"#80cdb3",
			"#269f4b",
			"#b0c1f0",
			"#9182c2",
			"#d9b4cd",
			"#b0832b",
			"#a2a9b1",
		],
		title: {
			text: "Pageviews Trend",
		},
		tooltip: {
			trigger: "axis",
		},
		legend: {
			top: "bottom",
			left: "center",
		},
		xAxis: {
			type: "category",
			data: [],
		},
		yAxis: {
			type: "value",
		},
		series: [],
		toolbox: {
			show: true,
			feature: {
				dataZoom: {
					yAxisIndex: "none",
				},
				dataView: { readOnly: false },
				magicType: { type: ["line", "bar"] },
				restore: {},
				saveAsImage: {},
			},
		},
	};

	chartInstance.setOption(initialOption);
	window.onresize = chartInstance.resize;
}

function updateChart(data, label) {
	if (!chartInstance) {
		initializeChart();
	}

	const existingOption = chartInstance.getOption();

	// Update xAxis data if new dates are present
	const newDates = data.map((item) => item.date);
	const existingDates = existingOption.xAxis[0].data;
	const mergedDates = Array.from(new Set([...existingDates, ...newDates]));
	mergedDates.sort();
	chartInstance.setOption({
		xAxis: {
			data: mergedDates,
		},
	});

	// Add a new series for the new data
	chartInstance.setOption({
		series: [
			...existingOption.series,
			{
				name: label,
				data: data.map((item) => item.views),
				type: "line",
				smooth: true,
			},
		],
	});
}

function renderCategories(categories, wiki) {
	const container = document.getElementById("category-list");
	container.innerHTML = "<h1>Categories</h1>";
	document.getElementById("article-list").innerHTML = "";

	const categoryListElement = document.createElement("ul");
	const lang = wiki.replaceAll("wiki", "");
	for (let i = 0; i < categories.length; i++) {
		const categoryElement = document.createElement("li");

		const categoryLink = document.createElement("a");
		categoryLink.href = "#";
		categoryLink.innerText = categories[i].category_title;
		categoryLink.id = categories[i].category_qid;
		categoryLink.title = `${categories[i].category_title_en}: ${categories[i].match_score}`;

		categoryLink.addEventListener("click", async function (e) {
			e.preventDefault();
			const categoryQid = this.id;
			showMessage(`Fetching articles for ${this.innerText}...`, "success");
			try {
				const articles = await listArticles(wiki, categoryQid);
				renderArticles(articles, lang);
			} catch (error) {
				console.error("Error fetching articles:", error);
				showMessage("Failed to fetch articles. Please try again.", "error");
			}
		});

		categoryElement.append(categoryLink);
		categoryListElement.append(categoryElement);
	}
	container.append(categoryListElement);
}

function renderArticles(articles, lang) {
	const container = document.getElementById("article-list");
	container.innerHTML = "<h1>Articles</h1>";

	if (!articles || articles.length === 0) {
		container.innerHTML = "<p>No articles found in this category.</p>";
		return;
	}

	const articleListElement = document.createElement("ul");

	for (let i = 0; i < articles.length; i++) {
		const articleElement = document.createElement("li");

		const articleLink = document.createElement("a");
		articleLink.href = `https://${lang}.wikipedia.org/wiki/${articles[i].title}`;
		articleLink.innerText = articles[i].title;
		articleLink.id = articles[i].qid;
		articleLink.title = `QID: ${articles[i].qid}`;

		articleElement.append(articleLink);
		articleListElement.append(articleElement);
	}

	container.append(articleListElement);
}

function showMessage(message, type) {
	const messageEl = document.getElementById("status");
	messageEl.classList.remove("error-message");
	messageEl.classList.remove("success-message");
	messageEl.classList.add(
		type === "error" ? "error-message" : "success-message",
	);
	messageEl.textContent = message;
}

async function searchCategory(wiki, query, match_threshold) {
	const apiUrl = `/api/search/categories?wiki=${wiki}&query=${encodeURIComponent(
		query,
	)}&match_threshold=${match_threshold}`;

	try {
		const startTime = performance.now();
		const response = await fetch(apiUrl);
		if (!response.ok) {
			throw new Error("Failed to fetch data");
		}

		const data = await response.json();
		const endTime = performance.now();
		const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
		showMessage(`Searched ${query} in ${timeTaken} seconds.`, "success");
		return data.categories;
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch category data. Please try again.", "error");
	}
}

async function listArticles(wiki, category_qid) {
	const apiUrl = `/api/list/articles?wiki=${wiki}&category_qid=${category_qid}`;

	try {
		const startTime = performance.now();
		const response = await fetch(apiUrl);
		if (!response.ok) {
			throw new Error("Failed to fetch articles in category");
		}

		const data = await response.json();
		const endTime = performance.now();
		const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
		showMessage(
			`Fetched ${data.articles.length} articles in ${timeTaken} seconds.`,
			"success",
		);
		return data.articles;
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch articles. Please try again.", "error");
		throw error;
	}
}

function populateFormFromQueryParams() {
	const urlParams = new URLSearchParams(window.location.search);

	const wiki = urlParams.get("wiki");
	const category = urlParams.get("category");
	const match_threshold = urlParams.get("match_threshold");

	if (wiki) {
		document.getElementById("wiki").value = wiki;
	}
	if (category) {
		document.getElementById("category").value = category.replaceAll("_", " ");
	}
	if (match_threshold) {
		document.getElementById("match_threshold").value = match_threshold;
	}

	if (wiki && category) {
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

document.addEventListener("DOMContentLoaded", function () {
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
