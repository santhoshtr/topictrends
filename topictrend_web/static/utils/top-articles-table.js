function formatTitle(title) {
	return (title || "").replaceAll("_", " ");
}

function getWikiCode(wiki) {
	return (wiki || "enwiki").replace("wiki", "");
}

const TABLE_STYLE_ID = "tt-top-articles-table-styles";

const TABLE_STYLES = `
.tt-top-articles-table {
	width: 100%;
	border-collapse: collapse;
	margin-top: var(--spacing-sm);
	background-color: var(--background-color-base);
	border: 1px solid var(--border-color-base);
}

.tt-top-articles-table th:last-child,
.tt-top-articles-table td:last-child {
	width: 56px;
}

.tt-top-articles-table th,
.tt-top-articles-table td {
	padding: var(--spacing-50) var(--spacing-75);
	border-bottom: 1px solid var(--border-color-base);
	text-align: left;
	vertical-align: middle;
}

.tt-top-articles-table th {
	font-weight: var(--font-weight-bold);
	background-color: var(--background-color-neutral-subtle);
}

.tt-top-articles-table th:not(:first-child):not(:last-child),
.tt-top-articles-table td:not(:first-child):not(:last-child) {
	text-align: right;
}

.tt-article-cell {
	display: grid;
	grid-template-columns: 72px 1fr;
	gap: var(--spacing-50);
	align-items: start;
}

.tt-article-image-link {
	display: block;
	line-height: 0;
}

.tt-article-image {
	width: 72px;
	height: 72px;
	object-fit: cover;
	object-position: center 33%;
	border-radius: var(--border-radius-base);
	background-color: var(--background-color-neutral-subtle);
}

.tt-article-details {
	display: grid;
	gap: var(--spacing-25);
	min-width: 0;
	padding-top: 2px;
}

.tt-article-title {
	color: var(--color-progressive);
	text-decoration: none;
	font-weight: var(--font-weight-semi-bold);
}

.tt-article-title:hover {
	text-decoration: underline;
}

.tt-article-meta {
	display: flex;
	align-items: center;
	gap: var(--spacing-50);
	flex-wrap: nowrap;
	overflow-x: auto;
	overflow-y: hidden;
	max-width: 100%;
	padding-bottom: var(--spacing-25);
    scrollbar-width: thin;
}

.tt-category-wrap {
	display: inline-flex;
	align-items: center;
	gap: var(--spacing-25);
	flex: 0 0 auto;
}

.tt-category-chip {
	display: inline-flex;
	align-items: center;
	gap: var(--spacing-25);
	padding: var(--spacing-25) var(--spacing-50);
	border-radius: var(--border-radius-base);
	background-color: var(--background-color-progressive-subtle);
	color: var(--color-progressive);
	text-decoration: none;
	font-size: var(--font-size-small);
	line-height: 1.2;
}

.tt-category-chip:hover {
	background-color: var(--background-color-progressive-subtle--hover);
}

.tt-category-trend-link {
	visibility: hidden;
	display: inline-flex;
	align-items: center;
	color: var(--color-progressive);
	text-decoration: none;
}

.tt-category-wrap:hover .tt-category-trend-link {
	visibility: visible;
}

.tt-number-cell {
	white-space: nowrap;
	font-variant-numeric: tabular-nums;
}

.tt-plot-cell {
	text-align: center;
	white-space: nowrap;
	vertical-align: middle;
}

.tt-row-plot-link {
	display: inline-flex;
	align-items: center;
	color: var(--color-base);
	text-decoration: none;
}

@media (max-width: 768px) {
	.tt-top-articles-table th,
	.tt-top-articles-table td {
		padding: var(--spacing-50);
	}

	.tt-article-cell {
		grid-template-columns: 56px 1fr;
	}

	.tt-article-image {
		width: 56px;
		height: 56px;
	}
}
`;

function ensureTableStyles(container) {
	if (container.querySelector(`#${TABLE_STYLE_ID}`)) {
		return;
	}
	const style = document.createElement("style");
	style.id = TABLE_STYLE_ID;
	style.textContent = TABLE_STYLES;
	container.appendChild(style);
}

function createCategoryPill(category, wiki, trendPath, startDate, endDate) {
	const categoryTitle = category?.title;
	const categoryQid = category?.qid;
	if (!categoryTitle) return null;

	const wrapper = document.createElement("div");
	wrapper.className = "tt-category-wrap";

	const params = new URLSearchParams({
		type: "category",
		wiki,
		category: categoryTitle.toString(),
		depth: "4",
	});
	if (categoryQid) {
		params.set("category_qid", categoryQid.toString());
	}
	if (startDate) params.set("start_date", startDate);
	if (endDate) params.set("end_date", endDate);

	const labelLink = document.createElement("a");
	labelLink.className = "tt-category-chip";
	labelLink.href = `/${trendPath}?${params.toString()}`;
	labelLink.innerHTML =
		'<svg height="14px" viewBox="0 -960 960 960" width="14px" fill="currentColor" aria-hidden="true" class="category-icon"><path d="M856-390 570-104q-12 12-27 18t-30 6q-15 0-30-6t-27-18L103-457q-11-11-17-25.5T80-513v-287q0-33 23.5-56.5T160-880h287q16 0 31 6.5t26 17.5l352 353q12 12 17.5 27t5.5 30q0 15-5.5 29.5T856-390ZM513-160l286-286-353-354H160v286l353 354ZM260-640q25 0 42.5-17.5T320-700q0-25-17.5-42.5T260-760q-25 0-42.5 17.5T200-700q0 25 17.5 42.5T260-640Zm220 160Z"></path></svg>';
	const text = document.createElement("span");
	text.textContent = formatTitle(categoryTitle.toString());
	labelLink.appendChild(text);

	const plotLink = document.createElement("a");
	plotLink.className = "tt-category-trend-link";
	plotLink.href = `/${trendPath}?${params.toString()}`;
	plotLink.title = "Plot category trend";
	plotLink.setAttribute(
		"aria-label",
		`Plot trend for ${formatTitle(categoryTitle.toString())}`,
	);
	plotLink.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" height="14px" viewBox="0 -960 960 960" width="14px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';

	wrapper.appendChild(labelLink);
	wrapper.appendChild(plotLink);
	return wrapper;
}

function createArticleCell(article, wiki, trendPath, startDate, endDate) {
	const wikiCode = getWikiCode(wiki);
	const cell = document.createElement("td");

	const articleWrap = document.createElement("div");
	articleWrap.className = "tt-article-cell";

	const imageLink = document.createElement("a");
	imageLink.href = `https://${wikiCode}.wikipedia.org/wiki/${encodeURIComponent(article.title)}`;
	imageLink.target = "_blank";
	imageLink.rel = "noopener noreferrer";
	imageLink.className = "tt-article-image-link";

	const image = document.createElement("img");
	image.className = "tt-article-image";
	image.loading = "lazy";
	image.alt = formatTitle(article.title);
	image.src = `https://wiki-display-image.toolforge.org/webp/${wikiCode}/${encodeURIComponent(article.title)}?width=200`;
	imageLink.appendChild(image);

	const details = document.createElement("div");
	details.className = "tt-article-details";

	const titleLink = document.createElement("a");
	titleLink.className = "tt-article-title";
	titleLink.textContent = formatTitle(article.title);
	titleLink.href = `https://${wikiCode}.wikipedia.org/wiki/${encodeURIComponent(article.title)}`;
	titleLink.target = "_blank";
	titleLink.rel = "noopener noreferrer";

	const meta = document.createElement("div");
	meta.className = "tt-article-meta";

	const categories = Array.isArray(article.categories)
		? article.categories
		: article.source_category_title || article.source_category_qid
			? [
					{
						title: article.source_category_title || article.source_category_qid,
						qid: article.source_category_qid,
					},
				]
			: [];

	for (const category of categories) {
		const categoryPill = createCategoryPill(
			category,
			wiki,
			trendPath,
			startDate,
			endDate,
		);
		if (categoryPill) {
			meta.appendChild(categoryPill);
		}
	}

	details.appendChild(titleLink);
	details.appendChild(meta);

	articleWrap.appendChild(imageLink);
	articleWrap.appendChild(details);
	cell.appendChild(articleWrap);
	return cell;
}

function createNumberCell(value, formatter = (v) => v.toLocaleString()) {
	const cell = document.createElement("td");
	cell.className = "tt-number-cell";
	cell.textContent = formatter(value);
	return cell;
}

function createPlotCell(article, wiki, trendPath, startDate, endDate) {
	const cell = document.createElement("td");
	cell.className = "tt-plot-cell";

	const trendLink = document.createElement("a");
	trendLink.className = "tt-row-plot-link";
	const trendParams = new URLSearchParams({
		type: "article",
		wiki,
		article: article.title,
	});
	if (startDate) trendParams.set("start_date", startDate);
	if (endDate) trendParams.set("end_date", endDate);
	trendLink.href = `/${trendPath}?${trendParams.toString()}`;
	trendLink.title = "Plot article trend";
	trendLink.setAttribute(
		"aria-label",
		`Plot trend for ${formatTitle(article.title)}`,
	);
	trendLink.innerHTML =
		'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';

	cell.appendChild(trendLink);
	return cell;
}

function createTable(headers) {
	const table = document.createElement("table");
	table.className = "tt-top-articles-table";

	const thead = document.createElement("thead");
	const row = document.createElement("tr");
	for (const header of headers) {
		const th = document.createElement("th");
		th.scope = "col";
		th.textContent = header;
		row.appendChild(th);
	}
	thead.appendChild(row);
	table.appendChild(thead);

	const tbody = document.createElement("tbody");
	table.appendChild(tbody);

	return { table, tbody };
}

export function renderPageviewsTopArticles(
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	container.innerHTML = "";
	ensureTableStyles(container);
	if (!topArticles?.length) return;

	const heading = document.createElement("h3");
	heading.textContent = "Top Articles";
	container.appendChild(heading);

	const { table, tbody } = createTable(["Article", "Views", "Plot"]);
	for (const article of topArticles) {
		const row = document.createElement("tr");
		row.appendChild(
			createArticleCell(article, wiki, "pageviews/trends", startDate, endDate),
		);
		row.appendChild(createNumberCell(article.views));
		row.appendChild(
			createPlotCell(article, wiki, "pageviews/trends", startDate, endDate),
		);
		tbody.appendChild(row);
	}

	container.appendChild(table);
}

export function renderPageeditsTopArticles(
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	container.innerHTML = "";
	ensureTableStyles(container);
	if (!topArticles?.length) return;

	const heading = document.createElement("h3");
	heading.textContent = "Top Articles";
	container.appendChild(heading);

	const { table, tbody } = createTable(["Article", "Edits", "Plot"]);
	for (const article of topArticles) {
		const row = document.createElement("tr");
		row.appendChild(
			createArticleCell(article, wiki, "pageedits/trends", startDate, endDate),
		);
		row.appendChild(createNumberCell(article.edits));
		row.appendChild(
			createPlotCell(article, wiki, "pageedits/trends", startDate, endDate),
		);
		tbody.appendChild(row);
	}

	container.appendChild(table);
}

export function renderGoogleSearchTopArticles(
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	container.innerHTML = "";
	ensureTableStyles(container);
	if (!topArticles?.length) return;

	const heading = document.createElement("h3");
	heading.textContent = "Top Articles";
	container.appendChild(heading);

	const { table, tbody } = createTable([
		"Article",
		"Clicks",
		"Impressions",
		"CTR",
		"Plot",
	]);
	for (const article of topArticles) {
		const row = document.createElement("tr");
		row.appendChild(
			createArticleCell(
				article,
				wiki,
				"googlesearch/trends",
				startDate,
				endDate,
			),
		);
		row.appendChild(createNumberCell(article.clicks));
		row.appendChild(createNumberCell(article.impressions));
		row.appendChild(
			createNumberCell(article.ctr, (v) => `${(v * 100).toFixed(2)}%`),
		);
		row.appendChild(
			createPlotCell(article, wiki, "googlesearch/trends", startDate, endDate),
		);
		tbody.appendChild(row);
	}

	container.appendChild(table);
}
