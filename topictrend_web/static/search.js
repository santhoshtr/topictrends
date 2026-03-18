import { hideProgress, showProgress } from "./utils/progress-bar.js";
import { showMessage } from "./utils/ui-utils.js";
import { populateWikiDropdown } from "./utils/wiki-utils.js";

const BASE_URL = "https://topictrends.wmcloud.org";

document.addEventListener("DOMContentLoaded", async () => {
	document.getElementById("search-form").addEventListener("submit", onSubmit);

	const wikiSelector = document.getElementById("wiki");
	const categoryElement = document.getElementById("category");

	categoryElement.setAttribute(
		"wiki",
		wikiSelector.value.replaceAll("wiki", ""),
	);

	await populateWikiDropdown();
	populateFormFromQueryParams();

	wikiSelector.addEventListener("change", updateTrendLinks);
	categoryElement.addEventListener("input", updateTrendLinks);
});

function updateTrendLinks() {
	const wiki = document.getElementById("wiki").value;
	const category = document
		.getElementById("category")
		.value.replaceAll(" ", "_");
	const params = new URLSearchParams({
		type: "category",
		wiki,
		depth: 4,
		category,
	});
	document.getElementById("pageviews-trends-link").href =
		`${BASE_URL}/pageviews/trends?${params}`;
	document.getElementById("pageedits-trends-link").href =
		`${BASE_URL}/pageedits/trends?${params}`;
	document.getElementById("googlesearch-trends-link").href =
		`${BASE_URL}/googlesearch/trends?${params}`;
}

async function onSubmit(event) {
	event.preventDefault();

	document.getElementById("category-list").style.display = "block";
	document.getElementById("article-list").style.display = "block";

	const wiki = document.getElementById("wiki").value;
	const match_threshold = document.getElementById("match_threshold").value;
	const params = new URLSearchParams({ wiki });

	try {
		const category = document
			.getElementById("category")
			.value.replaceAll(" ", "_");
		params.append("category", category);
		params.append("match_threshold", match_threshold);
		window.history.pushState({}, "", `${window.location.pathname}?${params}`);

		updateTrendLinks();
		const categories = await searchCategory(wiki, category, match_threshold);
		renderCategories(categories, wiki);
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch data. Please try again.", "error");
	}
}

function renderCategories(categories, wiki) {
	const container = document.getElementById("category-list");
	container.innerHTML = "<h1>Categories</h1>";
	document.getElementById("article-list").innerHTML = "";

	const lang = wiki.replaceAll("wiki", "");
	const list = document.createElement("ul");

	for (const cat of categories) {
		const li = document.createElement("li");
		const a = document.createElement("a");
		a.href = "#";
		a.innerText = cat.category_title;
		a.id = cat.category_qid;
		a.title = `${cat.category_title_en}: ${cat.match_score}`;
		a.addEventListener("click", async (e) => {
			e.preventDefault();
			showMessage(`Fetching articles for ${a.innerText}...`, "success");
			try {
				const articles = await listArticles(wiki, a.id);
				renderArticles(articles, lang);
			} catch (error) {
				console.error("Error fetching articles:", error);
				showMessage("Failed to fetch articles. Please try again.", "error");
			}
		});
		li.append(a);
		list.append(li);
	}
	container.append(list);
}

function renderArticles(articles, lang) {
	const container = document.getElementById("article-list");
	container.innerHTML = "<h1>Articles</h1>";

	if (!articles || articles.length === 0) {
		container.innerHTML = "<p>No articles found in this category.</p>";
		return;
	}

	const list = document.createElement("ul");
	for (const article of articles) {
		const li = document.createElement("li");
		const a = document.createElement("a");
		a.href = `https://${lang}.wikipedia.org/wiki/${article.title}`;
		a.innerText = article.title;
		a.id = article.qid;
		a.title = `QID: ${article.qid}`;
		li.append(a);
		list.append(li);
	}
	container.append(list);
}

async function searchCategory(wiki, query, match_threshold) {
	const apiUrl = `https://topictrends.wmcloud.org/api/search/categories?wiki=${wiki}&query=${encodeURIComponent(query)}&match_threshold=${match_threshold}`;

	try {
		showProgress();
		const startTime = performance.now();
		const response = await fetch(apiUrl);
		if (!response.ok) throw new Error("Failed to fetch data");
		const data = await response.json();
		const timeTaken = ((performance.now() - startTime) / 1000).toFixed(2);
		showMessage(`Searched ${query} in ${timeTaken} seconds.`, "success");
		return data.categories;
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch category data. Please try again.", "error");
	} finally {
		hideProgress();
	}
}

async function listArticles(wiki, category_qid) {
	const apiUrl = `https://topictrends.wmcloud.org/api/list/articles?wiki=${wiki}&category_qid=${category_qid}`;

	try {
		showProgress();
		const startTime = performance.now();
		const response = await fetch(apiUrl);
		if (!response.ok) throw new Error("Failed to fetch articles in category");
		const data = await response.json();
		const timeTaken = ((performance.now() - startTime) / 1000).toFixed(2);
		showMessage(
			`Fetched ${data.articles.length} articles in ${timeTaken} seconds.`,
			"success",
		);
		return data.articles;
	} catch (error) {
		console.error("Error:", error);
		showMessage("Failed to fetch articles. Please try again.", "error");
		throw error;
	} finally {
		hideProgress();
	}
}

function populateFormFromQueryParams() {
	const urlParams = new URLSearchParams(window.location.search);
	const wiki = urlParams.get("wiki");
	const category = urlParams.get("category");
	const match_threshold = urlParams.get("match_threshold");

	if (wiki) document.getElementById("wiki").value = wiki;
	if (category)
		document.getElementById("category").value = category.replaceAll("_", " ");
	if (match_threshold)
		document.getElementById("match_threshold").value = match_threshold;

	if (wiki && category) {
		updateTrendLinks();
		onSubmit(new Event("submit"));
	}
}
