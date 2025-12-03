const styleURL = new URL("./wiki-trends.css", import.meta.url);
class TopicTrends extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.articles = [];
    this.loading = false;
    this.error = null;
    this.wiki = this.getAttribute("wiki") || "enwiki";
  }

  static get observedAttributes() {
    return ["wiki"];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name === "wiki" && oldValue !== newValue) {
      this.wiki = newValue;
      this.fetchData();
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
      const url = `https://topictrends.wmcloud.org/api/list/top_categories?wiki=${this.wiki}&top_n=50`;
      const response = await fetch(url);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();

      // Process articles - flatten all articles from all categories
      this.articles = [];
      const articleMap = new Map(); // Use Map to deduplicate by title

      data.categories.forEach((category) => {
        category.top_articles.forEach((article) => {
          // If article already exists, add this category to it
          if (articleMap.has(article.title)) {
            const existingArticle = articleMap.get(article.title);
            if (!existingArticle.categories.includes(category.title)) {
              existingArticle.categories.push(category.title);
            }
          } else {
            // Create new article with its first category
            articleMap.set(article.title, {
              ...article,
              categories: [category.title],
            });
          }
        });
      });

      // Convert Map back to array and sort by views
      this.articles = Array.from(articleMap.values()).sort(
        (a, b) => b.views - a.views,
      );

      // Update stats
      const statsDisplay = document.getElementById("stats-display");
      if (statsDisplay) {
        const wikiCode = this.wiki.replace("wiki", "");
        statsDisplay.textContent = `Showing ${this.articles.length} unique articles from ${data.categories.length} top categories (${wikiCode} Wikipedia)`;
      }
    } catch (error) {
      console.error("Error fetching data:", error);
      this.error = error.message;
    } finally {
      this.loading = false;
      this.render();
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

  createArticleElement(article) {
    const wikiCode = this.wiki.replace("wiki", "");
    const imageUrl = `https://wiki-display-image.toolforge.org/webp/${wikiCode}/${encodeURIComponent(article.title)}?width=180`;

    const articleDiv = document.createElement("div");
    articleDiv.className = "article-item";

    // Create image
    const img = document.createElement("img");
    img.src = imageUrl;
    img.alt = this.formatTitle(article.title);
    img.className = "article-image";
    img.setAttribute("loading", "lazy");
    // Create content div
    const contentDiv = document.createElement("div");
    contentDiv.className = "article-content";

    // Create title
    const titleDiv = document.createElement("a");
    titleDiv.className = "article-title";
    titleDiv.textContent = this.formatTitle(article.title);
    titleDiv.href = `https://${wikiCode}.wikipedia.org/wiki/${article.title}`;

    // Create categories div
    const categoriesDiv = document.createElement("div");
    categoriesDiv.className = "categories";

    article.categories.forEach((cat) => {
      const categoryTag = document.createElement("span");
      categoryTag.className = "category-tag";
      categoryTag.textContent = this.formatTitle(cat);
      categoriesDiv.appendChild(categoryTag);
    });

    contentDiv.appendChild(titleDiv);
    contentDiv.appendChild(categoriesDiv);

    // Create views count div
    const viewsDiv = document.createElement("div");
    viewsDiv.className = "views-count";

    const viewsNumber = document.createElement("div");
    viewsNumber.className = "views-number";
    viewsNumber.textContent = this.formatViews(article.views);

    const viewsLabel = document.createElement("div");
    viewsLabel.className = "views-label";
    viewsLabel.textContent = "Views";

    viewsDiv.appendChild(viewsNumber);
    viewsDiv.appendChild(viewsLabel);

    // Assemble article item
    articleDiv.appendChild(img);
    articleDiv.appendChild(contentDiv);
    articleDiv.appendChild(viewsDiv);

    return articleDiv;
  }

  createArticlesListElement() {
    const articlesDiv = document.createElement("div");
    articlesDiv.className = "articles-list";

    this.articles.forEach((article) => {
      articlesDiv.appendChild(this.createArticleElement(article));
    });

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
    // Clear shadow root
    this.shadowRoot.innerHTML = "";

    // Load and add styles
    const style = document.createElement("style");
    style.textContent = `@import url(${styleURL});`;
    this.shadowRoot.appendChild(style);

    // Create container
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

// Define the custom element
customElements.define("wiki-trends", TopicTrends);
