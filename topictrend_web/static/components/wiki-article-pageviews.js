const styleURL = new URL("./wiki-article-pageviews.css", import.meta.url);

class WikiArticlePageviews extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
  }

  static get observedAttributes() {
    return ["wiki", "title", "views", "categories", "qid"];
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

  get views() {
    return parseInt(this.getAttribute("views") || "0");
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

  formatViews(views) {
    if (views >= 1000000) {
      return (views / 1000000).toFixed(1) + "M";
    } else if (views >= 1000) {
      return (views / 1000).toFixed(0) + "k";
    }
    return views.toString();
  }

  formatTitle(title) {
    return title.replace(/_/g, " ");
  }

  render() {
    const wikiCode = this.wiki.replace("wiki", "");
    const imageUrl = `https://wiki-display-image.toolforge.org/webp/${wikiCode}/${encodeURIComponent(this.title)}?width=180`;

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
      const categoryViews = typeof cat === "string" ? 0 : cat.views;

      categoryEl.setAttribute("title", categoryTitle);
      categoryEl.setAttribute("qid", categoryQid.toString());
      categoryEl.setAttribute("views", categoryViews.toString());

      categoriesDiv.appendChild(categoryEl);
    });

    contentDiv.appendChild(titleDiv);
    contentDiv.appendChild(categoriesDiv);

    const viewsDiv = document.createElement("div");
    viewsDiv.className = "views-count";

    const viewsNumber = document.createElement("div");
    viewsNumber.className = "views-number";
    viewsNumber.textContent = this.formatViews(this.views);

    const viewsLabel = document.createElement("div");
    viewsLabel.className = "views-label";
    viewsLabel.textContent = "Views";

    viewsDiv.appendChild(viewsNumber);
    viewsDiv.appendChild(viewsLabel);

    articleDiv.appendChild(img);
    articleDiv.appendChild(contentDiv);
    articleDiv.appendChild(viewsDiv);

    this.shadowRoot.appendChild(articleDiv);
  }
}

customElements.define("wiki-article-pageviews", WikiArticlePageviews);
