const styleURL = new URL("./wiki-article-pageviews.css", import.meta.url);

class WikiArticlePageedits extends HTMLElement {
	constructor() {
		super();
		this.attachShadow({ mode: "open" });
	}

	static get observedAttributes() {
		return ["wiki", "title", "metric", "categories", "qid"];
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

		const titleDiv = document.createElement("a");
		titleDiv.className = "article-title";
		titleDiv.textContent = this.formatTitle(this.title);
		titleDiv.href = `https://${wikiCode}.wikipedia.org/wiki/${this.title}`;
		titleDiv.target = "_blank";

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

			categoriesDiv.appendChild(categoryEl);
		});

		contentDiv.appendChild(titleDiv);
		contentDiv.appendChild(categoriesDiv);

		const editsDiv = document.createElement("div");
		editsDiv.className = "views-count";

		const editsNumber = document.createElement("a");
		editsNumber.className = "views-number";
		editsNumber.textContent = this.formatEdits(this.edits);
		const trendParams = new URLSearchParams({
			type: "article",
			wiki: this.wiki,
			article: this.title,
		});
		editsNumber.href = `/pageedits/trends?${trendParams}`;

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
