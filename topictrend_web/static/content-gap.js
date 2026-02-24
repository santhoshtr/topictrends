import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import { buildWikipediaUrl } from "./utils/wiki-utils.js";

// wikis already fetched (reused for compare-with picker)
let allWikis = [];
// wikis currently shown in results (base + added comparisons)
let activeWikis = [];
// current query state
let currentCategory = null;
let currentDepth = "2";

document.addEventListener("DOMContentLoaded", async () => {
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
		const wikisParam = [...new Set(["enwiki", ...activeWikis])].join(",");
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
		<p class="results-meta">
			Depth: ${data.depth} &nbsp;·&nbsp;
			Overlap (articles present in all wikis): <strong>${data.overlap_count}</strong>
		</p>
	`;

	wikiResults.innerHTML = "";

	const refCount = data.wikis[0]?.article_count ?? 0;

	data.wikis.forEach((wikiResult) => {
		const missing = data.missing_from[wikiResult.wiki];
		const card = document.createElement("div");
		card.className = "wiki-result-card";

		const missingCount = missing ? missing.count : 0;
		const coveragePercent =
			refCount > 0
				? Math.round((wikiResult.article_count / refCount) * 100)
				: 100;

		card.innerHTML = `
			<div class="wiki-card-header">
				<span class="wiki-code">${wikiResult.wiki}</span>
				<span class="article-count">${wikiResult.article_count} articles</span>
				${
					missingCount > 0
						? `<span class="missing-badge">${missingCount} missing</span>`
						: `<span class="complete-badge">complete</span>`
				}
			</div>
			<div class="coverage-bar-wrap">
				<div class="coverage-bar" style="width: ${coveragePercent}%"></div>
				<span class="coverage-label">${coveragePercent}% of ${data.wikis[0]?.wiki ?? "enwiki"}</span>
			</div>
			<div class="wiki-card-actions">
				<a class="cdx-button" href="${buildTrendsUrl("pageviews", wikiResult.wiki, data.category, currentDepth)}">Pageviews</a>
				<a class="cdx-button" href="${buildTrendsUrl("pageedits", wikiResult.wiki, data.category, currentDepth)}">Page edits</a>
			</div>
		`;

		if (missing && missing.article_qids.length > 0) {
			const details = document.createElement("details");
			details.className = "missing-articles";
			details.innerHTML = `<summary>${missingCount} articles missing from ${wikiResult.wiki}</summary>`;

			const ul = document.createElement("ul");
			ul.className = "missing-article-list";

			const refWiki = data.wikis.find((w) => w.wiki === "enwiki");
			missing.article_qids.slice(0, 50).forEach((qid) => {
				const refArticle = refWiki?.articles?.find((a) => a.qid === qid);
				const title = refArticle?.title ?? `QID ${qid}`;
				const li = document.createElement("li");
				if (refArticle) {
					li.innerHTML = `<a href="${buildWikipediaUrl("enwiki", title)}" target="_blank" rel="noopener">${title}</a>`;
				} else {
					li.textContent = title;
				}
				ul.appendChild(li);
			});

			if (missing.article_qids.length > 50) {
				const more = document.createElement("li");
				more.className = "more-indicator";
				more.textContent = `… and ${missing.article_qids.length - 50} more`;
				ul.appendChild(more);
			}

			details.appendChild(ul);
			card.appendChild(details);
		}

		wikiResults.appendChild(card);
	});

	wikiResults.appendChild(buildCompareRow());

	resultsSection.hidden = false;
}

function buildCompareRow() {
	const row = document.createElement("div");
	row.className = "compare-with-row";

	const label = document.createElement("span");
	label.className = "compare-with-label";
	label.textContent = "Compare with";

	const select = document.createElement("select");
	select.className = "cdx-select compare-with-select";

	const placeholder = document.createElement("option");
	placeholder.value = "";
	placeholder.textContent = "Choose a wiki…";
	select.appendChild(placeholder);

	allWikis
		.filter((w) => !activeWikis.includes(w.code))
		.forEach((wiki) => {
			const opt = document.createElement("option");
			opt.value = wiki.code;
			opt.textContent = `${wiki.langcode} — ${wiki.localname || wiki.name}`;
			select.appendChild(opt);
		});

	select.addEventListener("change", async () => {
		const chosen = select.value;
		if (!chosen) return;
		activeWikis.push(chosen);
		await fetchAndRender();
	});

	row.appendChild(label);
	row.appendChild(select);
	return row;
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
