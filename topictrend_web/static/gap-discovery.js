import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";

const form = document.getElementById("gap-discovery-form");
const referenceSelect = document.getElementById("reference-wiki");
const targetSelect = document.getElementById("target-wiki");
const resultsSection = document.getElementById("gap-results-section");
const resultsHeader = document.getElementById("results-header");
const resultsEl = document.getElementById("gap-results");
const pagination = document.getElementById("pagination");
const prevBtn = document.getElementById("prev-page");
const nextBtn = document.getElementById("next-page");
const pageRange = document.getElementById("page-range");
const statusEl = document.getElementById("status");

// Query state. `skipBase` is the floor set by the "Skip top categories" field;
// Prev/Next move `offset` in steps of `limit` but never below `skipBase`.
const state = {
	reference: "enwiki",
	target: "hiwiki",
	hasCategory: null, // null = all, true = under-populated, false = missing
	maxRef: null,
	limit: 50,
	skipBase: 0,
	offset: 0,
	total: 0,
};

function langCode(wiki) {
	return wiki.replace(/wiki$/, "");
}

function categoryUrl(wiki, title) {
	const t = encodeURIComponent(title.replace(/ /g, "_"));
	return `https://${langCode(wiki)}.wikipedia.org/wiki/Category:${t}`;
}

function wikidataUrl(qid) {
	return `https://www.wikidata.org/wiki/Q${qid}`;
}

async function loadWikiList() {
	try {
		const response = await fetch("/static/wikis.json");
		if (!response.ok) throw new Error(`HTTP ${response.status}`);
		const wikis = await response.json();
		for (const select of [referenceSelect, targetSelect]) {
			select.innerHTML = "";
			for (const wiki of wikis) {
				const option = document.createElement("option");
				option.value = wiki.code;
				option.textContent = `${wiki.langcode} - ${wiki.name}`;
				select.appendChild(option);
			}
		}
		referenceSelect.value = state.reference;
		targetSelect.value = state.target;
	} catch (error) {
		console.error("Failed to load wiki list:", error);
	}
}

function structureToHasCategory(value) {
	if (value === "missing") return false;
	if (value === "under") return true;
	return null;
}

function readForm() {
	state.reference = referenceSelect.value;
	state.target = targetSelect.value;
	const struct = form.querySelector('input[name="structure"]:checked')?.value ?? "all";
	state.hasCategory = structureToHasCategory(struct);
	const maxRefRaw = document.getElementById("max-ref").value.trim();
	state.maxRef = maxRefRaw === "" ? null : Math.max(0, Number.parseInt(maxRefRaw, 10) || 0);
	state.limit = Number.parseInt(document.getElementById("limit").value, 10) || 50;
	state.skipBase = Math.max(0, Number.parseInt(document.getElementById("skip").value, 10) || 0);
	state.offset = state.skipBase;
}

function syncUrl() {
	const p = new URLSearchParams({
		reference: state.reference,
		target: state.target,
		skip: String(state.offset),
		limit: String(state.limit),
	});
	if (state.maxRef != null) p.set("max_ref", String(state.maxRef));
	if (state.hasCategory === true) p.set("structure", "under");
	if (state.hasCategory === false) p.set("structure", "missing");
	window.history.replaceState({}, "", `${window.location.pathname}?${p}`);
}

function applyUrlParams() {
	const p = new URLSearchParams(window.location.search);
	if (p.has("reference")) state.reference = p.get("reference");
	if (p.has("target")) state.target = p.get("target");
	if (p.has("limit")) document.getElementById("limit").value = p.get("limit");
	if (p.has("skip")) document.getElementById("skip").value = p.get("skip");
	if (p.has("max_ref")) document.getElementById("max-ref").value = p.get("max_ref");
	const struct = p.get("structure");
	if (struct === "missing") document.getElementById("struct-missing").checked = true;
	else if (struct === "under") document.getElementById("struct-under").checked = true;
	return p.has("reference") && p.has("target");
}

async function fetchAndRender() {
	if (state.reference === state.target) {
		showMessage("Reference and target wikis must differ.", "error");
		return;
	}
	const params = new URLSearchParams({
		reference: state.reference,
		target: state.target,
		offset: String(state.offset),
		limit: String(state.limit),
	});
	if (state.maxRef != null) params.set("max_ref", String(state.maxRef));
	if (state.hasCategory != null) params.set("has_category", String(state.hasCategory));

	showProgress();
	statusEl.textContent = "";
	try {
		const response = await fetch(`/api/gap_discovery/categories?${params}`);
		if (!response.ok) {
			const body = await response.json().catch(() => ({}));
			throw new Error(body.error || `HTTP ${response.status}`);
		}
		const data = await response.json();
		state.total = data.total;
		renderResults(data);
		renderPagination(data);
		resultsSection.hidden = false;
		syncUrl();
	} catch (error) {
		showMessage(`Failed to load gaps: ${error.message}`, "error");
		resultsSection.hidden = true;
	} finally {
		hideProgress();
	}
}

function renderResults(data) {
	resultsHeader.innerHTML = "";
	const h2 = document.createElement("h2");
	h2.innerHTML = `Gaps: <em>${data.target}</em> vs <em>${data.reference}</em>`;
	const meta = document.createElement("p");
	meta.className = "results-meta";
	meta.textContent = `snapshot ${data.target_date} · ${data.total.toLocaleString()} gaps `
		+ `(${data.without_category.toLocaleString()} missing category, `
		+ `${data.with_category.toLocaleString()} under-populated)`;
	resultsHeader.append(h2, meta);

	if (data.categories.length === 0) {
		resultsEl.innerHTML = `<div class="empty-state">No gaps with these filters. Try
			lowering "Hide categories larger than", reducing "Skip top", or switching Gap type
			to All.</div>`;
		return;
	}

	const table = document.createElement("table");
	table.className = "gap-table";
	table.innerHTML = `<thead><tr>
		<th scope="col">#</th>
		<th scope="col">Category</th>
		<th scope="col" class="num">${data.reference}</th>
		<th scope="col" class="num">${data.target}</th>
		<th scope="col" class="num">Gap</th>
		<th scope="col">Coverage</th>
		<th scope="col">Category status</th>
	</tr></thead>`;
	const tbody = document.createElement("tbody");

	data.categories.forEach((c, i) => {
		const tr = document.createElement("tr");
		const rank = data.offset + i + 1;
		const cov = c.coverage_pct * 100;
		const covTier = cov < 10 ? "low" : cov < 50 ? "mid" : "high";
		const title = c.category_title.replace(/_/g, " ");
		const articlesNote = `${data.target} files ${c.direct_coverage_target.toLocaleString()} articles directly under this category`;
		const badge = c.has_category
			? `<span class="structure-badge exists">Present<span class="badge-sub" title="${articlesNote}">${c.direct_coverage_target.toLocaleString()} articles</span></span>`
			: `<span class="structure-badge absent" title="${data.target} has no such category">Absent</span>`;

		tr.innerHTML = `
			<td class="num rank">${rank}</td>
			<td class="cat">
				<a href="${categoryUrl(data.reference, c.category_title)}" target="_blank" rel="noopener noreferrer">${title}</a>
				<a class="qid-sub" href="${wikidataUrl(c.category_qid)}" target="_blank" rel="noopener noreferrer">Q${c.category_qid}</a>
			</td>
			<td class="num">${c.overlap_reference.toLocaleString()}</td>
			<td class="num">${c.overlap_target.toLocaleString()}</td>
			<td class="num">${c.gap.toLocaleString()}</td>
			<td class="cov-cell">
				<span class="cov-track" aria-hidden="true"><span class="cov-fill ${covTier}" style="width:${Math.min(cov, 100)}%"></span></span>
				<span class="cov-num">${cov.toFixed(1)}%</span>
			</td>
			<td>${badge}</td>`;
		tbody.appendChild(tr);
	});
	table.appendChild(tbody);
	resultsEl.replaceChildren(table);
}

function renderPagination(data) {
	const start = data.total === 0 ? 0 : data.offset + 1;
	const end = Math.min(data.offset + data.limit, data.total);
	pageRange.textContent = `rows ${start.toLocaleString()}–${end.toLocaleString()} of ${data.total.toLocaleString()} gaps`;
	prevBtn.disabled = state.offset <= state.skipBase;
	nextBtn.disabled = data.offset + data.limit >= data.total;
	pagination.hidden = data.total === 0;
}

form.addEventListener("submit", (e) => {
	e.preventDefault();
	readForm();
	fetchAndRender();
});

document.getElementById("hide-giants").addEventListener("click", () => {
	document.getElementById("max-ref").value = "5000";
});

prevBtn.addEventListener("click", () => {
	state.offset = Math.max(state.skipBase, state.offset - state.limit);
	fetchAndRender();
});

nextBtn.addEventListener("click", () => {
	state.offset += state.limit;
	fetchAndRender();
});

await loadWikiList();
if (applyUrlParams()) {
	referenceSelect.value = state.reference;
	targetSelect.value = state.target;
	readForm();
	fetchAndRender();
}
