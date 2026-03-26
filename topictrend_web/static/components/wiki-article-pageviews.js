import { renderPageviewsTopArticles } from "../utils/top-articles-table.js";

export function renderPageviewsArticlesTable(
	container,
	wiki,
	articles,
	startDate,
	endDate,
) {
	renderPageviewsTopArticles(container, wiki, articles, startDate, endDate);
}

class WikiArticlePageviews extends HTMLElement {
	connectedCallback() {
		this.style.display = "none";
	}
}

customElements.define("wiki-article-pageviews", WikiArticlePageviews);
