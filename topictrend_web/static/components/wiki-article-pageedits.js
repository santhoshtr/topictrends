import { renderPageeditsTopArticles } from "../utils/top-articles-table.js";

export function renderPageeditsArticlesTable(
	container,
	wiki,
	articles,
	startDate,
	endDate,
) {
	renderPageeditsTopArticles(container, wiki, articles, startDate, endDate);
}

class WikiArticlePageedits extends HTMLElement {
	connectedCallback() {
		this.style.display = "none";
	}
}

customElements.define("wiki-article-pageedits", WikiArticlePageedits);
