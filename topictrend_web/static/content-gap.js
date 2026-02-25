import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

// wikis already fetched (reused for compare-with picker)
let allWikis = [];
// wikis currently shown in results (base + added comparisons)
let activeWikis = [];
// current query state
let currentCategory = null;
let currentDepth = "2";

document.addEventListener("DOMContentLoaded", async () => {
	await populateWikiDropdown();
	allWikis = await loadWikiList();
	document
		.getElementById("content-gap-form")
		.addEventListener("submit", onSubmit);
	populateFormFromQueryParams();
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

	const category = document
		.getElementById("category")
		.value.trim()
		.replaceAll(" ", "_");
	if (!category) {
		showMessage("Please select a category.", "error");
		return;
	}

	const baseWiki = document.getElementById("wiki").value;
	const depth = document.getElementById("depth").value || "2";

	currentCategory = category;
	currentDepth = depth;
	activeWikis = [baseWiki];

	const params = new URLSearchParams({ category, wiki: baseWiki, depth });
	window.history.pushState({}, "", `${window.location.pathname}?${params}`);

	await fetchAndRender();
}

async function fetchAndRender() {
	const resultsSection = document.getElementById("content-gap-results");
	resultsSection.hidden = true;

	showProgress();
	try {
		const wikisParam = [...new Set(activeWikis)].join(",");
		const url = `/api/content_gap/categories?category=${encodeURIComponent(currentCategory)}&wikis=${encodeURIComponent(wikisParam)}&depth=${currentDepth}`;
		const response = await fetch(url);
		if (!response.ok) throw new Error(`HTTP ${response.status}`);
		const data = await response.json();
		hideProgress();
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
			</tr>
		</thead>
	`;

	const tbody = document.createElement("tbody");

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
			<td><a href="${buildTrendsUrl("pageviews", wikiResult.wiki, data.category, currentDepth)}">Pageviews</a></td>
			<td><a href="${buildTrendsUrl("pageedits", wikiResult.wiki, data.category, currentDepth)}">Page edits</a></td>
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

function buildSearchUrl(wiki, category, depth) {
	const params = new URLSearchParams({
		wiki,
		category,
		match_threshold: "0.6",
		depth,
	});
	return `/search?${params}`;
}

function buildTrendsUrl(page, wiki, category, depth) {
	const params = new URLSearchParams({
		type: "category",
		wiki,
		category,
		depth,
	});
	return `${page === "pageviews" ? "/pageviews/trends" : "/pageedits/trends"}?${params}`;
}

function populateFormFromQueryParams() {
	const p = new URLSearchParams(window.location.search);
	const category = p.get("category");
	const wiki = p.get("wiki");
	const depth = p.get("depth");

	if (category) {
		document.getElementById("category").value = category.replaceAll("_", " ");
	}
	if (wiki) {
		document.getElementById("wiki").value = wiki;
	}
	if (depth) {
		document.getElementById("depth").value = depth;
	}
	if (category) {
		currentCategory = category;
		currentDepth = depth || "2";
		activeWikis = [wiki || document.getElementById("wiki").value];
		fetchAndRender();
	}
}
