import { renderGoogleSearchTopArticles } from "../utils/top-articles-table.js";

export function renderSearchArticlesTable(
	container,
	wiki,
	articles,
	startDate,
	endDate,
) {
	renderGoogleSearchTopArticles(container, wiki, articles, startDate, endDate);
}

class WikiArticleSearch extends HTMLElement {
	connectedCallback() {
		this.style.display = "none";
	}
}

customElements.define("wiki-article-search", WikiArticleSearch);
