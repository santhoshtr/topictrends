import { formatDateToISO, getDaysAgo } from "./utils/date-utils.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

document.addEventListener("DOMContentLoaded", async () => {
	await populateWikiDropdown();

	const mainEl = document.querySelector(".main");
	const metric = mainEl?.dataset.metric || "pageviews";

	const form = document.getElementById("top-form");
	const startDateInput = document.getElementById("start_date");
	const endDateInput = document.getElementById("end_date");
	const statsDisplay = document.getElementById("stats-display");

	// Default date range: last 30 days
	startDateInput.value = formatDateToISO(getDaysAgo(30));
	endDateInput.value = formatDateToISO(new Date());

	form.addEventListener("submit", (e) => {
		e.preventDefault();
		showTopics(metric);
	});

	// Auto-load on page open
	showTopics(metric);

	document.addEventListener("topictrends:stats", (event) => {
		if (!statsDisplay) return;
		statsDisplay.textContent = event.detail?.text || "";
	});
});

function showTopics(metric) {
	const wiki = document.getElementById("wiki").value;
	const startDate = document.getElementById("start_date").value;
	const endDate = document.getElementById("end_date").value;

	const mainEl = document.querySelector(".main");

	let wikiTrends = mainEl.querySelector("wiki-trends");
	if (!wikiTrends) {
		wikiTrends = document.createElement("wiki-trends");
		mainEl.appendChild(wikiTrends);
	}

	wikiTrends.setAttribute("metric", metric);
	wikiTrends.setAttribute("wiki", wiki);
	wikiTrends.setAttribute("start_date", startDate);
	wikiTrends.setAttribute("end_date", endDate);
}
