import { autocomp } from "./autocomp.js";

document.addEventListener("DOMContentLoaded", async function () {
	document.getElementById("delta-form").addEventListener("submit", onSubmit);

	// Set up wiki selector change handler
	const wikiSelector = document.getElementById("wiki");
	// Initialize with current wiki value
	const wikiValue = wikiSelector.value.replaceAll("wiki", "");

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
			updateChart(data, "Category Delta Analysis");
			// Clear articles chart when new category data is loaded
			clearArticlesChart();
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

	const API_URL = `https://topictrends.wmcloud.org/api/delta/categories?${params.toString()}`;

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

	const API_URL = `https://topictrends.wmcloud.org/api/delta/articles?${params.toString()}`;

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

let categoryChartInstance = null;
let articlesChartInstance = null;

function updateChart(data, label) {
	const theme = window.matchMedia("(prefers-color-scheme: dark)").matches
		? "dark"
		: "light";
	const chartElement = document.getElementById("chart");
	categoryChartInstance = echarts.init(chartElement, theme, {
		renderer: "svg",
	});

	const categories = data.categories.map((item) => item.category_title);
	const deltaPercentages = data.categories.map((item) => item.delta_percentage);

	const option = {
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
			text: "Top 20 Most Changed Categories",
			left: "center",
			textStyle: {
				fontSize: 20,
				fontWeight: "bold",
			},
			padding: [10, 0, 20, 0],
		},
		grid: {
			left: "20%",
			right: "10%",
			top: "12%",
			bottom: "8%",
		},
		xAxis: {
			type: "value",
			name: "Pageview Change (%)",
			nameLocation: "middle",
			nameGap: 35,
			nameTextStyle: {
				fontSize: 14,
				fontWeight: "bold",
			},
			axisLine: {
				lineStyle: {
					color: "#333",
				},
			},
			splitLine: {
				lineStyle: {
					type: "dashed",
					color: "#e0e0e0",
				},
			},
		},
		yAxis: {
			type: "category",
			data: categories,
			inverse: true,
			axisLabel: {
				fontSize: 12,
				interval: 0,
			},
			axisLine: {
				lineStyle: {
					color: "#333",
				},
			},
		},
		series: [
			{
				type: "bar",
				data: deltaPercentages,
				itemStyle: {
					color: function (params) {
						// Color bars based on positive/negative values
						return params.value >= 0 ? "#269f4b" : "#fd7865";
					},
				},
				barWidth: "70%",
				markLine: {
					silent: true,
					symbol: "none",
					data: [
						{
							xAxis: 0,
							lineStyle: {
								color: "red",
								type: "dashed",
								width: 2,
							},
							label: {
								show: false,
							},
						},
					],
				},
			},
		],
		toolbox: {
			show: true,
			feature: {
				dataZoom: {
					yAxisIndex: "none",
				},
				dataView: { readOnly: false },
				restore: {},
				saveAsImage: {},
			},
		},

		tooltip: {
			trigger: "axis",
			axisPointer: {
				type: "shadow",
			},
			formatter: function (params) {
				const value = params[0].value.toFixed(2);
				return `<strong>${params[0].name}</strong><br/>Change: ${value}%`;
			},
		},
	};

	// Add click event handler for bars
	categoryChartInstance.on("click", async function (params) {
		const categoryIndex = params.dataIndex;
		const categoryItem = data.categories[categoryIndex];
		const categoryQid = categoryItem.category_qid;

		showMessage(`Loading articles for: ${categoryItem.category_title}`, "info");

		// Get form values for the article request
		const wiki = document.getElementById("wiki").value;
		const baselineStartDate = document.getElementById(
			"baseline_start_date",
		).value;
		const baselineEndDate = document.getElementById("baseline_end_date").value;
		const impactStartDate = document.getElementById("impact_start_date").value;
		const impactEndDate = document.getElementById("impact_end_date").value;
		const depth = document.getElementById("depth").value;

		try {
			const articlesData = await fetchArticleDeltaData(
				wiki,
				categoryQid,
				baselineStartDate,
				baselineEndDate,
				impactStartDate,
				impactEndDate,
				depth,
				20, // limit to top 20 articles
			);

			if (articlesData && articlesData.articles.length > 0) {
				updateArticlesChart(articlesData);
				showMessage(
					`Loaded ${articlesData.articles.length} articles for: ${categoryItem.category_title}`,
					"success",
				);
			} else {
				showMessage(
					`No articles found for: ${categoryItem.category_title}`,
					"info",
				);
				clearArticlesChart();
			}
		} catch (error) {
			console.error("Error loading articles:", error);
			showMessage(
				`Failed to load articles for: ${categoryItem.category_title}`,
				"error",
			);
		}
	});

	categoryChartInstance.setOption(option);
}

function updateArticlesChart(data) {
	const theme = window.matchMedia("(prefers-color-scheme: dark)").matches
		? "dark"
		: "light";
	const articlesChartElement = document.getElementById("articles-chart");
	articlesChartInstance = echarts.init(articlesChartElement, theme, {
		renderer: "svg",
	});

	const articles = data.articles.map((item) => item.article_title);
	const deltaPercentages = data.articles.map((item) => item.delta_percentage);

	const option = {
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
			text: `Top Articles in: ${data.category_title}`,
			left: "center",
			textStyle: {
				fontSize: 18,
				fontWeight: "bold",
			},
			padding: [10, 0, 20, 0],
		},
		grid: {
			left: "25%",
			right: "10%",
			top: "12%",
			bottom: "8%",
		},
		xAxis: {
			type: "value",
			name: "Pageview Change (%)",
			nameLocation: "middle",
			nameGap: 35,
			nameTextStyle: {
				fontSize: 14,
				fontWeight: "bold",
			},
			axisLine: {
				lineStyle: {
					color: "#333",
				},
			},
			splitLine: {
				lineStyle: {
					type: "dashed",
					color: "#e0e0e0",
				},
			},
		},
		yAxis: {
			type: "category",
			data: articles,
			inverse: true,
			axisLabel: {
				fontSize: 11,
				interval: 0,
				width: 200,
				overflow: "truncate",
			},
			axisLine: {
				lineStyle: {
					color: "#333",
				},
			},
		},
		series: [
			{
				type: "bar",
				data: deltaPercentages,
				itemStyle: {
					color: function (params) {
						// Color bars based on positive/negative values
						return params.value >= 0 ? "#269f4b" : "#fd7865";
					},
				},
				barWidth: "60%",
				markLine: {
					silent: true,
					symbol: "none",
					data: [
						{
							xAxis: 0,
							lineStyle: {
								color: "red",
								type: "dashed",
								width: 2,
							},
							label: {
								show: false,
							},
						},
					],
				},
			},
		],
		toolbox: {
			show: true,
			feature: {
				dataZoom: {
					yAxisIndex: "none",
				},
				dataView: { readOnly: false },
				restore: {},
				saveAsImage: {},
			},
		},
		tooltip: {
			trigger: "axis",
			axisPointer: {
				type: "shadow",
			},
			formatter: function (params) {
				const value = params[0].value.toFixed(2);
				const articleData = data.articles[params[0].dataIndex];
				return `<strong>${params[0].name}</strong><br/>
                Change: ${value}%<br/>
                Baseline: ${articleData.baseline_views.toLocaleString()}<br/>
                Impact: ${articleData.impact_views.toLocaleString()}`;
			},
		},
	};

	articlesChartInstance.setOption(option);

	// Make the articles chart visible
	articlesChartElement.style.display = "block";
}

function clearArticlesChart() {
	const articlesChartElement = document.getElementById("articles-chart");
	if (articlesChartInstance) {
		articlesChartInstance.dispose();
		articlesChartInstance = null;
	}
	articlesChartElement.style.display = "none";
}

document.addEventListener("DOMContentLoaded", function () {
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
