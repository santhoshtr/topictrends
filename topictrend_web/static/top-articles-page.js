import { formatDateToISO, getDaysAgo } from "./utils/date-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import {
	renderGoogleSearchTopArticles,
	renderPageeditsTopArticles,
	renderPageviewsTopArticles,
} from "./utils/top-articles-table.js";
import "./components/wiki-selector.js";

const CONFIG = {
	pageviews: {
		endpoint: "/api/pageviews/top_articles",
		render: renderPageviewsTopArticles,
		metricLabel: "views",
	},
	pageedits: {
		endpoint: "/api/pageedits/top_articles",
		render: renderPageeditsTopArticles,
		metricLabel: "edits",
	},
	googlesearch: {
		endpoint: "/api/googlesearch/top_articles",
		render: renderGoogleSearchTopArticles,
		metricLabel: "clicks",
	},
};

document.addEventListener("DOMContentLoaded", () => {
	const form = document.getElementById("top-form");
	const startDateInput = document.getElementById("start_date");
	const endDateInput = document.getElementById("end_date");
	const statsDisplay = document.getElementById("stats-display");
	const main = document.querySelector(".main");

	const metric = main?.dataset.metric || "pageviews";
	const config = CONFIG[metric] || CONFIG.pageviews;

	startDateInput.value = formatDateToISO(getDaysAgo(30));
	endDateInput.value = formatDateToISO(new Date());

	form.addEventListener("submit", (event) => {
		event.preventDefault();
		loadTopArticles();
	});

	// Permalink support: <form-filler> reads the URL params and fills the form
	// once the async components have settled, then fires this event.
	form.addEventListener("form-fill-complete", () => loadTopArticles());

	// If the URL already carries prefill params, defer to form-filler so we load
	// once with the shared values instead of fetching defaults first.
	const urlParams = new URLSearchParams(window.location.search);
	const hasPrefill = ["wiki", "start_date", "end_date"].some((key) =>
		urlParams.has(key),
	);
	if (!hasPrefill) {
		loadTopArticles();
	}

	function setStatus(text) {
		if (statsDisplay) statsDisplay.textContent = text;
	}

	async function loadTopArticles() {
		const wiki = document.getElementById("wiki").value;
		const startDate = startDateInput.value;
		const endDate = endDateInput.value;

		const params = new URLSearchParams({ wiki, top_n: "100" });
		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);

		// Keep the URL shareable: it always reflects the current selection.
		window.history.replaceState(
			{},
			"",
			`${window.location.pathname}?${params.toString()}`,
		);

		if (!main) return;

		try {
			showProgress();
			const response = await fetch(`${config.endpoint}?${params.toString()}`);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();
			config.render(main, wiki, data.articles || [], startDate, endDate);

			const wikiCode = wiki.replace("wiki", "");
			setStatus(
				`Showing ${data.articles?.length || 0} top articles (${wikiCode} Wikipedia) — sorted by ${config.metricLabel}`,
			);
		} catch (error) {
			console.error("Error fetching top articles:", error);
			main.innerHTML =
				'<div class="error" style="padding: 1em;">Failed to load top articles.</div>';
			setStatus("Failed to load top articles.");
		} finally {
			hideProgress();
		}
	}
});
