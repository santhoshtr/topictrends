const styleURL = new URL("./wiki-trends.css", import.meta.url);

import { hideProgress, showProgress } from "../utils/progress-bar.js";
import { renderPageeditsArticlesTable } from "./wiki-article-pageedits.js";
import { renderPageviewsArticlesTable } from "./wiki-article-pageviews.js";
import { renderSearchArticlesTable } from "./wiki-article-search.js";

const API_PATHS = {
	pageviews: "https://topictrends.wmcloud.org/api/pageviews/top_categories",
	pageedits: "https://topictrends.wmcloud.org/api/pageedits/top_categories",
	googlesearch:
		"https://topictrends.wmcloud.org/api/googlesearch/top_categories",
};

const METRIC_LABELS = {
	pageviews: "views",
	pageedits: "edits",
	googlesearch: "clicks",
};

function getArticleMetricValue(article, metric) {
	if (metric === "pageedits") return article.edits;
	if (metric === "googlesearch") return article.clicks;
	return article.views;
}

class TopicTrends extends HTMLElement {
	constructor() {
		super();
		this.attachShadow({ mode: "open" });
		this.articles = [];
		this.loading = false;
		this.error = null;
		this.wiki = this.getAttribute("wiki") || "enwiki";
		this.start_date = this.getAttribute("start_date");
		this.end_date = this.getAttribute("end_date");
		this.metric = this.getAttribute("metric") || "pageviews";
	}

	static get observedAttributes() {
		return ["wiki", "start_date", "end_date", "metric"];
	}

	attributeChangedCallback(name, oldValue, newValue) {
		if (oldValue !== newValue) {
			if (name === "wiki") {
				this.wiki = newValue;
			} else if (name === "start_date") {
				this.start_date = newValue;
			} else if (name === "end_date") {
				this.end_date = newValue;
			} else if (name === "metric") {
				this.metric = newValue;
			}
			if (this.wiki && this.start_date && this.end_date) {
				this.fetchData();
			}
		}
	}

	connectedCallback() {
		this.render();
	}

	async fetchData() {
		this.loading = true;
		this.error = null;
		this.render();

		try {
			showProgress();
			const basePath = API_PATHS[this.metric] || API_PATHS.pageviews;
			let url = `${basePath}?wiki=${this.wiki}&top_n=50`;

			if (this.start_date) {
				url += `&start_date=${this.start_date}`;
			}

			if (this.end_date) {
				url += `&end_date=${this.end_date}`;
			}

			const response = await fetch(url);

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();

			this.articles = [];
			const articleMap = new Map();

			data.categories.forEach((category) => {
				category.top_articles.forEach((article) => {
					const metricValue = getArticleMetricValue(article, this.metric);
					const existingArticle = articleMap.get(article.title);
					if (!existingArticle) {
						articleMap.set(article.title, {
							...article,
							metricValue,
							source_category_qid: category.qid,
							source_category_title: category.title,
							categories: [
								{
									qid: category.qid,
									title: category.title,
									metric: getArticleMetricValue(category, this.metric),
								},
							],
						});
					} else {
						if (metricValue > existingArticle.metricValue) {
							existingArticle.metricValue = metricValue;
							existingArticle.qid = article.qid;
							existingArticle.source_category_qid = category.qid;
							existingArticle.source_category_title = category.title;
						}
						if (
							!existingArticle.categories.some(
								(cat) => cat.title === category.title,
							)
						) {
							existingArticle.categories.push({
								qid: category.qid,
								title: category.title,
								metric: getArticleMetricValue(category, this.metric),
							});
						}
					}
				});
			});

			this.articles = Array.from(articleMap.values()).sort(
				(a, b) => b.metricValue - a.metricValue,
			);

			const statsDisplay = document.getElementById("stats-display");
			if (statsDisplay) {
				const wikiCode = this.wiki.replace("wiki", "");
				const label = METRIC_LABELS[this.metric] || "views";
				statsDisplay.textContent = `Showing ${this.articles.length} unique articles from ${data.categories.length} top categories (${wikiCode} Wikipedia) — sorted by ${label}`;
			}
		} catch (error) {
			console.error("Error fetching data:", error);
			this.error = error.message;
		} finally {
			this.loading = false;
			hideProgress();
			this.render();
		}
	}

	createLoadingElement() {
		const loadingDiv = document.createElement("div");
		loadingDiv.className = "loading";

		const loadingText = document.createElement("div");
		loadingText.textContent = "Loading trending articles...";

		loadingDiv.appendChild(loadingText);
		return loadingDiv;
	}

	createErrorElement() {
		const errorDiv = document.createElement("div");
		errorDiv.className = "error";

		const strong = document.createElement("strong");
		strong.textContent = "Error:";

		const errorText = document.createTextNode(` ${this.error}`);

		errorDiv.appendChild(strong);
		errorDiv.appendChild(errorText);
		return errorDiv;
	}

	createArticlesListElement() {
		const articlesDiv = document.createElement("div");
		articlesDiv.className = "articles-list";

		if (this.metric === "pageedits") {
			renderPageeditsArticlesTable(
				articlesDiv,
				this.wiki,
				this.articles,
				this.start_date,
				this.end_date,
			);
		} else if (this.metric === "googlesearch") {
			renderSearchArticlesTable(
				articlesDiv,
				this.wiki,
				this.articles,
				this.start_date,
				this.end_date,
			);
		} else {
			renderPageviewsArticlesTable(
				articlesDiv,
				this.wiki,
				this.articles,
				this.start_date,
				this.end_date,
			);
		}

		return articlesDiv;
	}

	createEmptyStateElement() {
		const emptyDiv = document.createElement("div");
		emptyDiv.className = "empty-state";
		emptyDiv.textContent =
			"No articles found. Try selecting a different language.";
		return emptyDiv;
	}

	async render() {
		this.shadowRoot.innerHTML = "";

		const style = document.createElement("style");
		style.textContent = `@import url(${styleURL});`;
		this.shadowRoot.appendChild(style);

		const container = document.createElement("div");
		container.className = "container";

		if (this.loading) {
			container.appendChild(this.createLoadingElement());
		}

		if (this.error) {
			container.appendChild(this.createErrorElement());
		}

		if (!this.loading && !this.error) {
			if (this.articles.length > 0) {
				container.appendChild(this.createArticlesListElement());
			} else {
				container.appendChild(this.createEmptyStateElement());
			}
		}

		this.shadowRoot.appendChild(container);
	}
}

customElements.define("wiki-trends", TopicTrends);
