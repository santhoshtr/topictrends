import { formatDateToISO, getDaysAgo } from "./utils/date-utils.js";
import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { renderPageviewsTopArticles } from "./utils/top-articles-table.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

document.addEventListener("DOMContentLoaded", async () => {
	await populateWikiDropdown();

	const form = document.getElementById("top-form");
	const startDateInput = document.getElementById("start_date");
	const endDateInput = document.getElementById("end_date");
	const statsDisplay = document.getElementById("stats-display");

	startDateInput.value = formatDateToISO(getDaysAgo(30));
	endDateInput.value = formatDateToISO(new Date());

	form.addEventListener("submit", (event) => {
		event.preventDefault();
		loadTopArticles();
	});

	loadTopArticles();

	function setStatus(text) {
		if (statsDisplay) {
			statsDisplay.textContent = text;
		}
	}

	async function loadTopArticles() {
		const wiki = document.getElementById("wiki").value;
		const startDate = startDateInput.value;
		const endDate = endDateInput.value;

		const params = new URLSearchParams({
			wiki,
			top_n: "50",
		});

		if (startDate) params.set("start_date", startDate);
		if (endDate) params.set("end_date", endDate);

		const container = document.querySelector(".main");
		if (!container) return;

		try {
			showProgress();
			const response = await fetch(
				`https://topictrends.wmcloud.org/api/pageviews/top_articles?${params.toString()}`,
			);
			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();
			renderPageviewsTopArticles(
				container,
				wiki,
				data.articles || [],
				startDate,
				endDate,
			);

			const wikiCode = wiki.replace("wiki", "");
			setStatus(
				`Showing ${data.articles?.length || 0} top articles (${wikiCode} Wikipedia) — sorted by views`,
			);
		} catch (error) {
			console.error("Error fetching top articles:", error);
			container.innerHTML =
				'<div class="error" style="padding: 1em;">Failed to load top articles.</div>';
			setStatus("Failed to load top articles.");
		} finally {
			hideProgress();
		}
	}
});
