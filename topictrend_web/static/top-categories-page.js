import { formatDateToISO, getDaysAgo } from "./utils/date-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { buildWikipediaUrl } from "./utils/wiki-utils.js";
import "./components/wiki-selector.js";

const CONFIG = {
	pageviews: {
		endpoint: "/api/pageviews/top_categories",
		categoryTrendPath: "/pageviews/trends",
		articleTrendPath: "/pageviews/trends",
		categoryMetricField: "views",
		articleMetricField: "views",
		metricLabel: "views",
	},
	pageedits: {
		endpoint: "/api/pageedits/top_categories",
		categoryTrendPath: "/pageedits/trends",
		articleTrendPath: "/pageedits/trends",
		categoryMetricField: "edits",
		articleMetricField: "edits",
		metricLabel: "edits",
	},
	googlesearch: {
		endpoint: "/api/googlesearch/top_categories",
		categoryTrendPath: "/googlesearch/trends",
		articleTrendPath: "/googlesearch/trends",
		categoryMetricField: "clicks",
		articleMetricField: "clicks",
		metricLabel: "clicks",
	},
};

document.addEventListener("DOMContentLoaded", () => {
	const form = document.getElementById("top-form");
	const startDateInput = document.getElementById("start_date");
	const endDateInput = document.getElementById("end_date");
	const topNInput = document.getElementById("top_n");
	const results = document.getElementById("results");
	const status = document.getElementById("status");
	const main = document.querySelector(".main");

	const metric = main?.dataset.metric || "pageviews";
	const config = CONFIG[metric] || CONFIG.pageviews;

	startDateInput.value = formatDateToISO(getDaysAgo(30));
	endDateInput.value = formatDateToISO(new Date());

	form.addEventListener("submit", (event) => {
		event.preventDefault();
		loadTopCategories();
	});

	loadTopCategories();

	function formatTitle(title) {
		return (title || "").replaceAll("_", " ");
	}

	function setStatusText(text) {
		if (status) status.textContent = text;
	}

	function createTrendUrl(
		basePath,
		type,
		wiki,
		title,
		qid,
		startDate,
		endDate,
	) {
		const params = new URLSearchParams({ type, wiki });
		if (type === "category") {
			params.set("category", title);
			if (qid) params.set("category_qid", qid.toString());
		} else {
			params.set("article", title);
		}
		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);
		return `${basePath}?${params.toString()}`;
	}

	function renderCategories(wiki, categories, startDate, endDate) {
		if (!results) return;
		results.innerHTML = "";

		if (!categories?.length) {
			results.innerHTML =
				'<div class="empty-state"><p>No categories found for this range.</p></div>';
			return;
		}

		for (const category of categories) {
			const details = document.createElement("details");
			details.className = "category-accordion";

			const summary = document.createElement("summary");
			summary.className = "category-summary";

			const title = document.createElement("span");
			title.className = "category-name";
			title.textContent = formatTitle(category.title);

			const metricValue = document.createElement("span");
			metricValue.className = "category-views";
			metricValue.textContent = Number(
				category[config.categoryMetricField] || 0,
			).toLocaleString();

			const plotLink = document.createElement("a");
			plotLink.className = "plot-button";
			plotLink.href = createTrendUrl(
				config.categoryTrendPath,
				"category",
				wiki,
				category.title,
				category.qid,
				startDate,
				endDate,
			);
			plotLink.title = "Plot category trend";
			plotLink.innerHTML =
				'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
			plotLink.addEventListener("click", (event) => event.stopPropagation());

			summary.appendChild(title);
			summary.appendChild(metricValue);
			summary.appendChild(plotLink);
			details.appendChild(summary);

			const articleList = document.createElement("div");
			articleList.className = "article-list";

			for (const article of category.top_articles || []) {
				const row = document.createElement("div");
				row.className = "article-row";

				const articleLink = document.createElement("a");
				articleLink.className = "article-title";
				articleLink.href = buildWikipediaUrl(wiki, article.title);
				articleLink.target = "_blank";
				articleLink.rel = "noopener noreferrer";
				articleLink.textContent = formatTitle(article.title);

				const articleMetric = document.createElement("span");
				articleMetric.className = "article-views";
				articleMetric.textContent = Number(
					article[config.articleMetricField] || 0,
				).toLocaleString();

				const articlePlot = document.createElement("a");
				articlePlot.className = "article-plot-link";
				articlePlot.href = createTrendUrl(
					config.articleTrendPath,
					"article",
					wiki,
					article.title,
					null,
					startDate,
					endDate,
				);
				articlePlot.title = "Plot article trend";
				articlePlot.innerHTML =
					'<svg xmlns="http://www.w3.org/2000/svg" height="14px" viewBox="0 -960 960 960" width="14px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';

				const articleInfo = document.createElement("wiki-article-info");
				articleInfo.setAttribute("title", article.title);
				articleInfo.setAttribute("wiki", wiki);

				row.appendChild(articleLink);
				row.appendChild(articleInfo);
				row.appendChild(articleMetric);
				row.appendChild(articlePlot);
				articleList.appendChild(row);
			}

			details.appendChild(articleList);
			results.appendChild(details);
		}
	}

	async function loadTopCategories() {
		const wiki = document.getElementById("wiki").value;
		const startDate = startDateInput.value;
		const endDate = endDateInput.value;
		const topN = topNInput.value || "100";

		const params = new URLSearchParams({ wiki, top_n: topN });
		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);

		try {
			showProgress();
			const response = await fetch(`${config.endpoint}?${params.toString()}`);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();
			renderCategories(wiki, data.categories || [], startDate, endDate);

			const wikiCode = wiki.replace("wiki", "");
			setStatusText(
				`Showing ${data.categories?.length || 0} top categories (${wikiCode} Wikipedia) — sorted by ${config.metricLabel}`,
			);
		} catch (error) {
			console.error("Error fetching top categories:", error);
			if (results) {
				results.innerHTML =
					'<div class="empty-state"><p>Failed to load top categories.</p></div>';
			}
			setStatusText("Failed to load top categories.");
		} finally {
			hideProgress();
		}
	}
});
