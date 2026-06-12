const TABLE_CONFIG = {
	pageviews: {
		headers: ["Article", "Info", "Views", "Plot"],
		trendPath: "pageviews/trends",
		metricColumns: [{ field: "views" }],
	},
	pageedits: {
		headers: ["Article", "Info", "Edits", "Plot"],
		trendPath: "pageedits/trends",
		metricColumns: [{ field: "edits" }],
	},
	googlesearch: {
		headers: ["Article", "Info", "Clicks", "Impressions", "CTR", "Plot"],
		trendPath: "googlesearch/trends",
		metricColumns: [
			{ field: "clicks" },
			{ field: "impressions" },
			{ field: "ctr", formatter: (value) => `${(value * 100).toFixed(2)}%` },
		],
	},
};

function formatTitle(title) {
	return (title || "").replaceAll("_", " ");
}

function getWikiCode(wiki) {
	return (wiki || "enwiki").replace("wiki", "");
}

function buildCategoryParams(category, wiki, startDate, endDate) {
	const params = new URLSearchParams({
		type: "category",
		wiki,
		category: category.title.toString(),
	});
	if (category.qid) {
		params.set("category_qid", category.qid.toString());
	}
	if (startDate) params.set("start_date", startDate);
	if (endDate) params.set("end_date", endDate);
	return params;
}

function normalizeArticleCategories(article) {
	// source_categories from topic/category trends
	if (
		Array.isArray(article.source_categories) &&
		article.source_categories.length > 0
	) {
		return article.source_categories;
	}
	// categories from global top articles
	if (Array.isArray(article.categories)) {
		return article.categories;
	}
	return [];
}

function createCategoryPill(category, wiki, trendPath, startDate, endDate) {
	if (!category?.title) return null;

	const wrapper = document.createElement("div");
	wrapper.className = "tt-category-wrap";

	const params = buildCategoryParams(category, wiki, startDate, endDate);

	const labelLink = document.createElement("a");
	labelLink.className = "tt-category-chip";
	labelLink.href = `/${trendPath}?${params.toString()}`;
	labelLink.innerHTML =
		'<svg height="14px" viewBox="0 -960 960 960" width="14px" fill="currentColor" aria-hidden="true" class="category-icon"><path d="M856-390 570-104q-12 12-27 18t-30 6q-15 0-30-6t-27-18L103-457q-11-11-17-25.5T80-513v-287q0-33 23.5-56.5T160-880h287q16 0 31 6.5t26 17.5l352 353q12 12 17.5 27t5.5 30q0 15-5.5 29.5T856-390ZM513-160l286-286-353-354H160v286l353 354ZM260-640q25 0 42.5-17.5T320-700q0-25-17.5-42.5T260-760q-25 0-42.5 17.5T200-700q0 25 17.5 42.5T260-640Zm220 160Z"></path></svg>';
	const text = document.createElement("span");
	text.textContent = formatTitle(category.title.toString());
	labelLink.appendChild(text);

	const plotLink = document.createElement("a");
	plotLink.className = "tt-category-trend-link";
	plotLink.href = `/${trendPath}?${params.toString()}`;
	plotLink.title = "Plot category trend";
	plotLink.setAttribute(
		"aria-label",
		`Plot trend for ${formatTitle(category.title.toString())}`,
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
	image.src = `https://wiki-display-image.toolforge.org/webp/${wikiCode}/${encodeURIComponent(article.title)}?width=250`;
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

	for (const category of normalizeArticleCategories(article)) {
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

function createInfoCell(article, wiki) {
	const cell = document.createElement("td");
	cell.className = "tt-info-cell";
	const el = document.createElement("wiki-article-info");
	el.setAttribute("title", article.title);
	el.setAttribute("wiki", wiki);
	cell.appendChild(el);
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

function renderTopArticlesTable(
	metric,
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	container.innerHTML = "";
	if (!topArticles?.length) return;

	const config = TABLE_CONFIG[metric];
	const heading = document.createElement("h3");
	heading.textContent = "Top Articles";
	container.appendChild(heading);

	const { table, tbody } = createTable(config.headers);
	for (const article of topArticles) {
		const row = document.createElement("tr");
		row.appendChild(
			createArticleCell(article, wiki, config.trendPath, startDate, endDate),
		);
		row.appendChild(createInfoCell(article, wiki));
		for (const column of config.metricColumns) {
			row.appendChild(
				createNumberCell(article[column.field], column.formatter),
			);
		}
		row.appendChild(
			createPlotCell(article, wiki, config.trendPath, startDate, endDate),
		);
		tbody.appendChild(row);
	}

	container.appendChild(table);
}

export function renderPageviewsTopArticles(
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	renderTopArticlesTable(
		"pageviews",
		container,
		wiki,
		topArticles,
		startDate,
		endDate,
	);
}

export function renderPageeditsTopArticles(
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	renderTopArticlesTable(
		"pageedits",
		container,
		wiki,
		topArticles,
		startDate,
		endDate,
	);
}

export function renderGoogleSearchTopArticles(
	container,
	wiki,
	topArticles,
	startDate,
	endDate,
) {
	renderTopArticlesTable(
		"googlesearch",
		container,
		wiki,
		topArticles,
		startDate,
		endDate,
	);
}
