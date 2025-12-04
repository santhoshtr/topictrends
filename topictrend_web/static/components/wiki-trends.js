const styleURL = new URL("./wiki-trends.css", import.meta.url);
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
  }

  static get observedAttributes() {
    return ["wiki", "start_date", "end_date"];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue !== newValue) {
      if (name === "wiki") {
        this.wiki = newValue;
      } else if (name === "start_date") {
        this.start_date = newValue;
      } else if (name === "end_date") {
        this.end_date = newValue;
      }
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
      let url = `https://topictrends.wmcloud.org/api/list/top_categories?wiki=${this.wiki}&top_n=50`;
      
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
          if (articleMap.has(article.title)) {
            const existingArticle = articleMap.get(article.title);
            if (
              !existingArticle.categories.some(
                (cat) => cat.title === category.title,
              )
            ) {
              existingArticle.categories.push({
                qid: category.qid,
                title: category.title,
                views: category.views,
              });
            }
          } else {
            articleMap.set(article.title, {
              ...article,
              categories: [
                {
                  qid: category.qid,
                  title: category.title,
                  views: category.views,
                },
              ],
            });
          }
        });
      });

      this.articles = Array.from(articleMap.values()).sort(
        (a, b) => b.views - a.views,
      );

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
    const articleEl = document.createElement("wiki-article-pageviews");
    articleEl.setAttribute("wiki", this.wiki);
    articleEl.setAttribute("title", article.title);
    articleEl.setAttribute("views", article.views.toString());
    articleEl.setAttribute("qid", article.qid.toString());
    articleEl.setAttribute("categories", JSON.stringify(article.categories));
    return articleEl;
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
