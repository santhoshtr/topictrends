import { DEFAULT_CHART_COLORS } from "./utils/chart-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import "./components/wiki-selector.js";

// wikis already fetched (reused for compare-with picker)
let allWikis = [];
// wikis currently shown in results (base + added comparisons)
let activeWikis = [];
// current query state
let currentType = "topic"; // "topic" or "category"
let currentTopic = null;
let currentCategory = null;
let currentDepth = "2";
// last fetched data (for chart rendering)
let lastData = null;
// echarts instance for the article count chart
let chartInstance = null;

document.addEventListener("DOMContentLoaded", async () => {
	allWikis = await loadWikiList();
	document
		.getElementById("content-gap-form")
		.addEventListener("submit", onSubmit);
	document
		.getElementById("close-chart-dialog")
		.addEventListener("click", () =>
			document.getElementById("article-count-chart-dialog").close(),
		);
	document
		.getElementById("article-count-chart-dialog")
		.addEventListener("click", (e) => {
			if (e.target === e.currentTarget) e.currentTarget.close();
		});

	document.getElementById("content-gap-form").addEventListener("form-fill-complete", () => {
		const p = new URLSearchParams(window.location.search);
		const type = p.get("type") || "topic";
		const wiki = p.get("wiki");
		const depth = p.get("depth");
		const compare = p.get("compare");

		// Set the type tab (param name differs from field id: tab-topic / tab-category)
		const typeRadio = document.getElementById(`tab-${type}`);
		if (typeRadio) typeRadio.checked = true;

		if (type === "topic") {
			const topic = p.get("topic");
			if (topic) {
				currentType = "topic";
				currentTopic = topic;
				currentCategory = null;
			}
		} else {
			const category = p.get("category");
			if (category) {
				currentType = "category";
				currentCategory = category;
				currentTopic = null;
			}
		}

		if ((type === "topic" && currentTopic) || (type === "category" && currentCategory)) {
			currentDepth = depth || "2";
			const baseWiki = wiki || document.getElementById("wiki").value;
			const compareWikis = compare
				? compare.split(",").map((w) => w.trim()).filter(Boolean)
				: [];
			activeWikis = [baseWiki, ...compareWikis];
			fetchAndRender();
		}
	});

	if (!window.location.search) {
		document.querySelector(".examples").hidden = false;
	}
});

async function loadWikiList() {
	try {
		const response = await fetch("/static/wikis.json");
		if (!response.ok) throw new Error(`HTTP ${response.status}`);
		return await response.json();
	} catch (e) {
		console.error("Failed to load wiki list:", e);
		return [];
	}
}

async function onSubmit(event) {
	event.preventDefault();

	const type = document.querySelector('input[name="type"]:checked').value;
	const baseWiki = document.getElementById("wiki").value;
	const depth = document.getElementById("depth").value || "2";

	if (type === "topic") {
		const topic = document
			.getElementById("topic")
			.value.trim()
			.replaceAll(" ", "_");
		if (!topic) {
			showMessage("Please enter a topic.", "error");
			return;
		}

		currentType = "topic";
		currentTopic = topic;
		currentCategory = null;
	} else {
		const category = document
			.getElementById("category")
			.value.trim()
			.replaceAll(" ", "_");
		if (!category) {
			showMessage("Please select a category.", "error");
			return;
		}

		currentType = "category";
		currentCategory = category;
		currentTopic = null;
	}

	currentDepth = depth;
	// On a fresh submit keep only the base wiki; any previous compare wikis are discarded.
	activeWikis = [baseWiki];

	syncUrlParams();
	await fetchAndRender();
}

async function fetchAndRender() {
	const resultsSection = document.getElementById("content-gap-results");
	resultsSection.hidden = true;

	showProgress();
	try {
		const wikisParam = [...new Set(activeWikis)].join(",");
		let url;

		if (currentType === "topic") {
			url = `https://topictrends.wmcloud.org/api/content_gap/topic?topic=${encodeURIComponent(
				currentTopic,
			)}&wikis=${encodeURIComponent(wikisParam)}&depth=${currentDepth}`;
		} else {
			url = `https://topictrends.wmcloud.org/api/content_gap/categories?category=${encodeURIComponent(
				currentCategory,
			)}&wikis=${encodeURIComponent(wikisParam)}&depth=${currentDepth}`;
		}

		const response = await fetch(url);
		if (!response.ok) throw new Error(`HTTP ${response.status}`);
		const data = await response.json();
		hideProgress();
		lastData = data;
		renderResults(data);
	} catch (error) {
		hideProgress();
		showMessage(`Failed to fetch content gap data: ${error.message}`, "error");
	}
}

function renderResults(data) {
	const resultsSection = document.getElementById("content-gap-results");
	const header = document.getElementById("results-header");
	const wikiResults = document.getElementById("wiki-results");

	header.innerHTML = `
		<h2>Content gap: <em>${data.category.replaceAll("_", " ")}</em></h2>
		<p class="results-meta">Depth: ${data.depth}</p>
	`;

	const plotBtn = document.createElement("button");
	plotBtn.id = "plot-btn";
	plotBtn.className = "cdx-button";
	plotBtn.textContent = "Plot";
	plotBtn.disabled = activeWikis.length < 2;
	plotBtn.addEventListener("click", openChartDialog);
	header.appendChild(plotBtn);

	wikiResults.innerHTML = "";

	const table = document.createElement("table");
	table.className = "wiki-results-table";
	table.innerHTML = `
		<thead>
			<tr>
				<th>Wiki</th>
				<th>Articles</th>
				<th>Pageviews</th>
				<th>Page edits</th>
				<th>Google Search</th>
			</tr>
		</thead>
	`;

	const tbody = document.createElement("tbody");
	const plotIcon =
		'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
	data.wikis.forEach((wikiResult) => {
		const tr = document.createElement("tr");
		const searchUrl = buildSearchUrl(
			wikiResult.wiki,
			data.category,
			currentDepth,
		);
		tr.innerHTML = `
			<td class="wiki-code">${wikiResult.wiki}</td>
			<td><a href="${searchUrl}">${wikiResult.article_count} articles</a></td>
			<td><a href="${buildTrendsUrl("pageviews", wikiResult.wiki, data.category, currentDepth)}">${plotIcon}</a></td>
			<td><a href="${buildTrendsUrl("pageedits", wikiResult.wiki, data.category, currentDepth)}">${plotIcon}</a></td>
            <td><a href="${buildTrendsUrl("googlesearch", wikiResult.wiki, data.category, currentDepth)}">${plotIcon}</a></td>
		`;
		tbody.appendChild(tr);
	});

	tbody.appendChild(buildCompareRow());
	table.appendChild(tbody);
	wikiResults.appendChild(table);

	resultsSection.hidden = false;
}

function buildCompareRow() {
	const tr = document.createElement("tr");
	tr.className = "compare-with-row";

	const tdSelect = document.createElement("td");

	const select = document.createElement("select");
	select.className = "cdx-select compare-with-select";

	const placeholder = document.createElement("option");
	placeholder.value = "";
	placeholder.textContent = "Add a wiki…";
	select.appendChild(placeholder);

	allWikis
		.filter((w) => !activeWikis.includes(w.code))
		.forEach((wiki) => {
			const opt = document.createElement("option");
			opt.value = wiki.code;
			opt.textContent = `${wiki.langcode} - ${wiki.name}`;
			select.appendChild(opt);
		});

	select.addEventListener("change", async () => {
		const chosen = select.value;
		if (!chosen) return;
		activeWikis.push(chosen);
		syncUrlParams();
		await fetchAndRender();
	});

	tdSelect.appendChild(select);
	tr.appendChild(tdSelect);

	// fill remaining columns
	for (let i = 0; i < 3; i++) {
		tr.appendChild(document.createElement("td"));
	}

	return tr;
}

function openChartDialog() {
	const dialog = document.getElementById("article-count-chart-dialog");
	document.getElementById("chart-dialog-title").textContent =
		`Articles per wiki — ${lastData.category.replaceAll("_", " ")}`;
	dialog.showModal();
	renderChart(lastData);
}

function renderChart(data) {
	const el = document.getElementById("article-count-chart");
	const theme = window.matchMedia("(prefers-color-scheme: dark)").matches
		? "dark"
		: "light";

	if (!chartInstance) {
		chartInstance = echarts.init(el, theme, { renderer: "svg" });
		window.addEventListener("resize", () => chartInstance.resize());
	}

	// Base wiki (index 0) gets the first palette color; compare wikis get the rest.
	const wikis = data.wikis.map((w) => w.wiki);
	const counts = data.wikis.map((w) => w.article_count);
	const colors = data.wikis.map(
		(_, i) => DEFAULT_CHART_COLORS[i % DEFAULT_CHART_COLORS.length],
	);

	chartInstance.setOption(
		{
			color: DEFAULT_CHART_COLORS,
			title: {
				text: data.category.replaceAll("_", " "),
				subtext: currentType === "topic" ? "Topic" : "Category",
				left: "center",
				top: 0,
				textStyle: { fontSize: 13 },
				subtextStyle: { fontSize: 11 },
			},
			tooltip: {
				trigger: "axis",
				axisPointer: { type: "shadow" },
				formatter: (params) =>
					`${params[0].name}: <strong>${params[0].value}</strong> articles`,
			},
			grid: {
				left: "3%",
				right: "12%",
				top: "18%",
				bottom: "5%",
				containLabel: true,
			},
			xAxis: {
				type: "value",
				name: "Articles",
				nameLocation: "end",
			},
			yAxis: {
				type: "category",
				data: wikis,
				axisLabel: { fontFamily: "monospace", fontWeight: "bold" },
				inverse: true,
			},
			series: [
				{
					type: "bar",
					data: counts.map((val, i) => ({
						value: val,
						itemStyle: { color: colors[i] },
					})),
					label: { show: true, position: "right" },
				},
			],
		},
		true, // notMerge — full replace on each call
	);

	// ECharts needs a resize after showModal since the element was display:none.
	requestAnimationFrame(() => chartInstance.resize());
}

function syncUrlParams() {
	const baseWiki = activeWikis[0];
	const compareWikis = activeWikis.slice(1);
	const params = new URLSearchParams({
		type: currentType,
		wiki: baseWiki,
		depth: currentDepth,
	});

	if (currentType === "topic") {
		params.set("topic", currentTopic);
	} else {
		params.set("category", currentCategory);
	}

	if (compareWikis.length > 0) {
		params.set("compare", compareWikis.join(","));
	}
	window.history.pushState({}, "", `${window.location.pathname}?${params}`);
}

function buildSearchUrl(wiki, query, depth) {
	const params = new URLSearchParams({
		wiki,
		category: query,
		match_threshold: "0.6",
		depth,
	});
	return `/search?${params}`;
}

function buildTrendsUrl(page, wiki, query, depth) {
	const params = new URLSearchParams({
		type: currentType,
		wiki,
		depth,
	});

	if (currentType === "topic") {
		params.set("topic", query);
	} else {
		params.set("category", query);
	}

	return `/${page}/trends?${params}`;
}
