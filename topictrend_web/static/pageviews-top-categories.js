import { formatDateToISO, getDaysAgo } from "./utils/date-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { buildWikipediaUrl, populateWikiDropdown } from "./utils/wiki-utils.js";

document.addEventListener("DOMContentLoaded", async () => {
	await populateWikiDropdown();

	const form = document.getElementById("top-form");
	const startDateInput = document.getElementById("start_date");
	const endDateInput = document.getElementById("end_date");
	const topNInput = document.getElementById("top_n");
	const results = document.getElementById("results");
	const status = document.getElementById("status");

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

	function getWikiCode(wiki) {
		return (wiki || "enwiki").replace("wiki", "");
	}

	function setStatusText(text) {
		if (status) status.textContent = text;
	}

	function createCategoryTrendUrl(
		wiki,
		category,
		categoryQid,
		startDate,
		endDate,
	) {
		const params = new URLSearchParams({
			type: "category",
			wiki,
			depth: "4",
			category,
		});
		if (categoryQid) params.set("category_qid", categoryQid.toString());
		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);
		return `/pageviews/trends?${params.toString()}`;
	}

	function createArticleTrendUrl(wiki, article, startDate, endDate) {
		const params = new URLSearchParams({
			type: "article",
			wiki,
			article,
		});
		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);
		return `/pageviews/trends?${params.toString()}`;
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

			const views = document.createElement("span");
			views.className = "category-views";
			views.textContent = Number(category.views || 0).toLocaleString();

			const plotLink = document.createElement("a");
			plotLink.className = "plot-button";
			plotLink.href = createCategoryTrendUrl(
				wiki,
				category.title,
				category.qid,
				startDate,
				endDate,
			);
			plotLink.title = "Plot category trend";
			plotLink.setAttribute(
				"aria-label",
				`Plot trend for ${formatTitle(category.title)}`,
			);
			plotLink.innerHTML =
				'<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';
			plotLink.addEventListener("click", (event) => {
				event.stopPropagation();
			});

			summary.appendChild(title);
			summary.appendChild(views);
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

				const articleViews = document.createElement("span");
				articleViews.className = "article-views";
				articleViews.textContent = Number(article.views || 0).toLocaleString();

				const articlePlot = document.createElement("a");
				articlePlot.className = "article-plot-link";
				articlePlot.href = createArticleTrendUrl(
					wiki,
					article.title,
					startDate,
					endDate,
				);
				articlePlot.title = "Plot article trend";
				articlePlot.innerHTML =
					'<svg xmlns="http://www.w3.org/2000/svg" height="14px" viewBox="0 -960 960 960" width="14px" fill="currentColor" aria-hidden="true"><path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/></svg>';

				row.appendChild(articleLink);
				row.appendChild(articleViews);
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
		const topN = topNInput.value || "50";

		const params = new URLSearchParams({ wiki, top_n: topN });
		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);

		try {
			showProgress();
			const response = await fetch(
				`https://topictrends.wmcloud.org/api/pageviews/top_categories?${params.toString()}`,
			);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();
			renderCategories(wiki, data.categories || [], startDate, endDate);

			const wikiCode = wiki.replace("wiki", "");
			setStatusText(
				`Showing ${data.categories?.length || 0} top categories (${wikiCode} Wikipedia) — sorted by views`,
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
