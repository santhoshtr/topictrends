const styleURL = new URL("./wiki-article-pageviews.css", import.meta.url);

class WikiArticlePageedits extends HTMLElement {
	constructor() {
		super();
		this.attachShadow({ mode: "open" });
	}

	static get observedAttributes() {
		return [
			"wiki",
			"title",
			"metric",
			"categories",
			"qid",
			"start_date",
			"end_date",
		];
	}

	connectedCallback() {
		this.render();
	}

	attributeChangedCallback() {
		this.render();
	}

	get wiki() {
		return this.getAttribute("wiki") || "enwiki";
	}

	get title() {
		return this.getAttribute("title") || "";
	}

	get edits() {
		return parseInt(this.getAttribute("metric") || "0");
	}

	get qid() {
		return this.getAttribute("qid") || "";
	}

	get start_date() {
		return this.getAttribute("start_date") || "";
	}

	get end_date() {
		return this.getAttribute("end_date") || "";
	}

	get categories() {
		const categoriesAttr = this.getAttribute("categories");
		if (!categoriesAttr) return [];
		try {
			return JSON.parse(categoriesAttr);
		} catch {
			return [];
		}
	}

	formatEdits(edits) {
		if (edits >= 1000000) {
			return (edits / 1000000).toFixed(1) + "M";
		} else if (edits >= 1000) {
			return (edits / 1000).toFixed(0) + "k";
		}
		return edits.toString();
	}

	formatTitle(title) {
		return title.replace(/_/g, " ");
	}

	render() {
		const wikiCode = this.wiki.replace("wiki", "");
		const imageUrl = `https://wiki-display-image.toolforge.org/webp/${wikiCode}/${encodeURIComponent(this.title)}?width=250`;

		this.shadowRoot.innerHTML = "";

		const style = document.createElement("style");
		style.textContent = `@import url(${styleURL});`;
		this.shadowRoot.appendChild(style);

		const articleDiv = document.createElement("div");
		articleDiv.className = "article-item";

		const img = document.createElement("img");
		img.src = imageUrl;
		img.alt = this.formatTitle(this.title);
		img.className = "article-image";
		img.setAttribute("loading", "lazy");

		const contentDiv = document.createElement("div");
		contentDiv.className = "article-content";

		// Title row: Wikipedia link + hover trend icon
		const titleRow = document.createElement("div");
		titleRow.className = "article-title-row";

		const titleLink = document.createElement("a");
		titleLink.className = "article-title";
		titleLink.textContent = this.formatTitle(this.title);
		titleLink.href = `https://${wikiCode}.wikipedia.org/wiki/${this.title}`;
		titleLink.target = "_blank";
		titleLink.rel = "noopener noreferrer";

		const trendParams = new URLSearchParams({
			type: "article",
			wiki: this.wiki,
			article: this.title,
		});
		if (this.start_date) trendParams.set("start_date", this.start_date);
		if (this.end_date) trendParams.set("end_date", this.end_date);

		const trendLink = document.createElement("a");
		trendLink.className = "article-trend-link";
		trendLink.href = `/pageedits/trends?${trendParams}`;
		trendLink.title = "View edit trend";
		trendLink.setAttribute(
			"aria-label",
			`View edit trend for ${this.formatTitle(this.title)}`,
		);
		trendLink.textContent = "📉";

		titleRow.appendChild(titleLink);
		titleRow.appendChild(trendLink);

		const categoriesDiv = document.createElement("div");
		categoriesDiv.className = "categories";

		this.categories.forEach((cat) => {
			const categoryEl = document.createElement("wiki-category");
			const categoryTitle = typeof cat === "string" ? cat : cat.title;
			const categoryQid = typeof cat === "string" ? "" : cat.qid;
			const categoryMetric = typeof cat === "string" ? 0 : (cat.metric ?? 0);

			categoryEl.setAttribute("title", categoryTitle);
			categoryEl.setAttribute("qid", categoryQid.toString());
			categoryEl.setAttribute("views", categoryMetric.toString());
			categoryEl.setAttribute("wiki", this.wiki);
			categoryEl.setAttribute("trend_path", "pageedits/trends");
			if (this.start_date)
				categoryEl.setAttribute("start_date", this.start_date);
			if (this.end_date) categoryEl.setAttribute("end_date", this.end_date);

			categoriesDiv.appendChild(categoryEl);
		});

		contentDiv.appendChild(titleRow);
		contentDiv.appendChild(categoriesDiv);

		// Metric badge — plain text, no link
		const editsDiv = document.createElement("div");
		editsDiv.className = "views-count";

		const editsNumber = document.createElement("span");
		editsNumber.className = "views-number";
		editsNumber.textContent = this.formatEdits(this.edits);

		const editsLabel = document.createElement("div");
		editsLabel.className = "views-label";
		editsLabel.textContent = "Edits";

		editsDiv.appendChild(editsNumber);
		editsDiv.appendChild(editsLabel);

		articleDiv.appendChild(img);
		articleDiv.appendChild(contentDiv);
		articleDiv.appendChild(editsDiv);

		this.shadowRoot.appendChild(articleDiv);
	}
}

customElements.define("wiki-article-pageedits", WikiArticlePageedits);
