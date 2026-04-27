const styleURL = new URL("./wiki-article-info.css", import.meta.url);

const USER_AGENT = "TopicTrends/1.0 (https://topictrends.wmcloud.org)";

// SVG icons (Material Symbols, matching the rest of the codebase)
const ICON_INFO =
	'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="M440-280h80v-240h-80v240Zm40-320q17 0 28.5-11.5T520-640q0-17-11.5-28.5T480-680q-17 0-28.5 11.5T440-640q0 17 11.5 28.5T480-600Zm0 520q-83 0-156-31.5T197-197q-54-54-85.5-127T80-480q0-83 31.5-156T197-763q54-54 127-85.5T480-880q83 0 156 31.5T763-763q54 54 85.5 127T880-480q0 83-31.5 156T763-197q-54 54-127 85.5T480-80Zm0-80q134 0 227-93t93-227q0-134-93-227t-227-93q-134 0-227 93t-93 227q0 134 93 227t227 93Zm0-320Z"/></svg>';

const ICON_CLOSE =
	'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m256-200-56-56 224-224-224-224 56-56 224 224 224-224 56 56-224 224 224 224-56 56-224-224-224 224Z"/></svg>';

const ICON_LOADING =
	'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true" class="tt-spin"><path d="M480-80q-82 0-155-31.5t-127.5-86Q143-252 111.5-325T80-480q0-83 31.5-155.5t86-127Q252-817 325-848.5T480-880q17 0 28.5 11.5T520-840q0 17-11.5 28.5T480-800q-133 0-226.5 93.5T160-480q0 133 93.5 226.5T480-160q133 0 226.5-93.5T800-480q0-17 11.5-28.5T840-520q17 0 28.5 11.5T880-480q0 82-31.5 155t-86 127.5Q707-143 634.5-111.5T480-80Z"/></svg>';

// Class → CSS modifier mapping (lowercase key)
const CLASS_CSS = {
	fa: "fa",
	ga: "ga",
	a: "a",
	b: "b",
	c: "c",
	start: "start",
	stub: "stub",
};

function wikiLang(wiki) {
	// "enwiki" → "en", "zh_minnanwiki" → "zh_minnan", "simplewiki" → "simple"
	return (wiki || "enwiki").replace(/wiki$/, "");
}

function formatDate(isoString) {
	if (!isoString) return "—";
	const d = new Date(isoString);
	if (Number.isNaN(d.getTime())) return isoString;
	return d.toISOString().slice(0, 10); // YYYY-MM-DD
}

function classBadge(cls) {
	if (!cls || cls === "-" || cls === "") return "—";
	const key = cls.toLowerCase();
	const mod = CLASS_CSS[key] || "c";
	return `<span class="tt-class-badge tt-class-${mod}">${cls}</span>`;
}

async function fetchArticleInfo(title, lang) {
	const url = new URL(`https://${lang}.wikipedia.org/w/api.php`);
	url.search = new URLSearchParams({
		action: "query",
		titles: title,
		prop: "revisions|info|pageassessments",
		rvdir: "newer",
		rvlimit: "1",
		rvprop: "timestamp",
		format: "json",
		origin: "*",
	});

	const response = await fetch(url, {
		headers: { "Api-User-Agent": USER_AGENT },
	});
	if (!response.ok) throw new Error(`HTTP ${response.status}`);
	const data = await response.json();

	const pages = data?.query?.pages ?? {};
	const page = Object.values(pages)[0];
	if (!page) throw new Error("No page data returned");

	return {
		created: page.revisions?.[0]?.timestamp ?? null,
		lastModified: page.touched ?? null,
		assessments: page.pageassessments ?? null,
	};
}

function buildPopoverContent(title, state, data, error) {
	const displayTitle = (title || "").replaceAll("_", " ");

	const header = `
		<div class="tt-info-header">
			<span class="tt-info-title" title="${displayTitle}">${displayTitle}</span>
			<button class="tt-info-close-btn" popovertarget="wiki-article-info-popover"
				popovertargetaction="hide" aria-label="Close">
				${ICON_CLOSE}
			</button>
		</div>`;

	if (state === "loading") {
		return `${header}
		<div class="tt-info-loading">${ICON_LOADING} Loading…</div>`;
	}

	if (state === "error") {
		return `${header}
		<div class="tt-info-error">Failed to load article info: ${error}</div>`;
	}

	// Metadata section
	const metaSection = `
		<div class="tt-info-section">
			<div class="tt-info-section-heading">Metadata</div>
			<dl class="tt-info-dl">
				<dt class="tt-info-dt">Date created</dt>
				<dd class="tt-info-dd">${formatDate(data.created)}</dd>
				<dt class="tt-info-dt">Last modified</dt>
				<dd class="tt-info-dd">${formatDate(data.lastModified)}</dd>
			</dl>
		</div>`;

	// Assessments section
	let assessmentsBody;
	const entries = data.assessments ? Object.entries(data.assessments) : [];
	if (entries.length === 0) {
		assessmentsBody = `<p class="tt-info-dt">No assessment data available.</p>`;
	} else {
		const rows = entries
			.map(
				([project, info]) => `
				<tr>
					<td>${project}</td>
					<td>${classBadge(info.class)}</td>
					<td>${info.importance || "—"}</td>
				</tr>`,
			)
			.join("");
		assessmentsBody = `
			<table class="tt-info-table">
				<thead>
					<tr>
						<th>WikiProject</th>
						<th>Class</th>
						<th>Importance</th>
					</tr>
				</thead>
				<tbody>${rows}</tbody>
			</table>`;
	}

	const assessSection = `
		<div class="tt-info-section">
			<div class="tt-info-section-heading">Article Assessments</div>
			${assessmentsBody}
		</div>`;

	return `${header}${metaSection}${assessSection}`;
}

class WikiArticleInfo extends HTMLElement {
	// Shared singleton popover across all instances
	static #popover = null;
	// Currently active instance that owns the popover content
	static #activeInstance = null;

	static #getPopover() {
		if (WikiArticleInfo.#popover) return WikiArticleInfo.#popover;

		const el = document.createElement("div");
		el.id = "wiki-article-info-popover";
		el.setAttribute("popover", "auto");
		document.body.appendChild(el);
		WikiArticleInfo.#popover = el;
		return el;
	}

	// Per-instance fetch cache keyed by "lang:title"
	#cache = new Map();

	connectedCallback() {
		// Inject stylesheet once into document head
		const styleId = "wiki-article-info-styles";
		if (!document.getElementById(styleId)) {
			const link = document.createElement("link");
			link.id = styleId;
			link.rel = "stylesheet";
			link.href = styleURL.href;
			document.head.appendChild(link);
		}

		// Spin up shared popover element
		WikiArticleInfo.#getPopover();

		// Render trigger button
		const btn = document.createElement("button");
		btn.className = "tt-info-btn";
		btn.type = "button";
		btn.setAttribute("aria-label", "Article info");
		btn.innerHTML = ICON_INFO;
		btn.addEventListener("click", () => this.#onClick());
		this.innerHTML = "";
		this.appendChild(btn);
	}

	async #onClick() {
		const title = this.getAttribute("title") || "";
		const wiki = this.getAttribute("wiki") || "enwiki";
		const lang = wikiLang(wiki);
		const cacheKey = `${lang}:${title}`;

		const popover = WikiArticleInfo.#getPopover();
		WikiArticleInfo.#activeInstance = this;

		// Show immediately with loading state
		popover.innerHTML = buildPopoverContent(title, "loading", null, null);
		popover.showPopover();

		// Return cached result without re-fetching
		if (this.#cache.has(cacheKey)) {
			const cached = this.#cache.get(cacheKey);
			if (WikiArticleInfo.#activeInstance === this) {
				popover.innerHTML = buildPopoverContent(
					title,
					cached.ok ? "data" : "error",
					cached.data,
					cached.error,
				);
			}
			return;
		}

		try {
			const data = await fetchArticleInfo(title, lang);
			this.#cache.set(cacheKey, { ok: true, data });
			if (WikiArticleInfo.#activeInstance === this) {
				popover.innerHTML = buildPopoverContent(title, "data", data, null);
			}
		} catch (err) {
			const msg = err?.message ?? String(err);
			this.#cache.set(cacheKey, { ok: false, error: msg });
			if (WikiArticleInfo.#activeInstance === this) {
				popover.innerHTML = buildPopoverContent(title, "error", null, msg);
			}
		}
	}
}

customElements.define("wiki-article-info", WikiArticleInfo);
