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
      const categoryTag = document.createElement("span");
      categoryTag.className = "category-tag";

      const iconSvg = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "svg",
      );
      iconSvg.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      iconSvg.setAttribute("height", "14px");
      iconSvg.setAttribute("viewBox", "0 -960 960 960");
      iconSvg.setAttribute("width", "14px");
      iconSvg.setAttribute("fill", "currentColor");
      iconSvg.classList.add("category-icon");

      const path = document.createElementNS(
        "http://www.w3.org/2000/svg",
        "path",
      );
      path.setAttribute(
        "d",
        "M856-390 570-104q-12 12-27 18t-30 6q-15 0-30-6t-27-18L103-457q-11-11-17-25.5T80-513v-287q0-33 23.5-56.5T160-880h287q16 0 31 6.5t26 17.5l352 353q12 12 17.5 27t5.5 30q0 15-5.5 29.5T856-390ZM513-160l286-286-353-354H160v286l353 354ZM260-640q25 0 42.5-17.5T320-700q0-25-17.5-42.5T260-760q-25 0-42.5 17.5T200-700q0 25 17.5 42.5T260-640Zm220 160Z",
      );

      iconSvg.appendChild(path);
      categoryTag.appendChild(iconSvg);

      const textSpan = document.createElement("span");
      const categoryTitle = typeof cat === "string" ? cat : cat.title;
      textSpan.textContent = this.formatTitle(categoryTitle);
      categoryTag.title = `Category ${categoryTitle} Q${cat.qid}: ${cat.views} pageviews`;
      categoryTag.appendChild(textSpan);

      categoryTag.addEventListener("click", () => {
        const categoryInput = document.getElementById("category");
        const categoryQidInput = document.getElementById("category_qid");

        if (categoryInput) {
          categoryInput.value = categoryTitle;
        }

        if (categoryQidInput) {
          categoryQidInput.value = cat.qid;
        }
      });

      categoriesDiv.appendChild(categoryTag);
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
