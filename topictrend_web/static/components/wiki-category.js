const styleURL = new URL("./wiki-category.css", import.meta.url);

class WikiCategory extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
  }

  static get observedAttributes() {
    return ["title", "qid", "views"];
  }

  connectedCallback() {
    this.render();
  }

  attributeChangedCallback() {
    this.render();
  }

  get title() {
    return this.getAttribute("title") || "";
  }

  get qid() {
    return this.getAttribute("qid") || "";
  }

  get views() {
    return parseInt(this.getAttribute("views") || "0");
  }

  formatTitle(title) {
    return title.replace(/_/g, " ");
  }

  formatViews(views) {
    if (views >= 1000000) {
      return (views / 1000000).toFixed(1) + "M";
    } else if (views >= 1000) {
      return (views / 1000).toFixed(0) + "k";
    }
    return views.toString();
  }

  render() {
    this.shadowRoot.innerHTML = "";

    const style = document.createElement("style");
    style.textContent = `@import url(${styleURL});`;
    this.shadowRoot.appendChild(style);

    const categoryTag = document.createElement("span");
    categoryTag.className = "category-tag";
    categoryTag.title = `Category ${this.title} Q${this.qid}: ${this.formatViews(this.views)} pageviews`;

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

    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute(
      "d",
      "M856-390 570-104q-12 12-27 18t-30 6q-15 0-30-6t-27-18L103-457q-11-11-17-25.5T80-513v-287q0-33 23.5-56.5T160-880h287q16 0 31 6.5t26 17.5l352 353q12 12 17.5 27t5.5 30q0 15-5.5 29.5T856-390ZM513-160l286-286-353-354H160v286l353 354ZM260-640q25 0 42.5-17.5T320-700q0-25-17.5-42.5T260-760q-25 0-42.5 17.5T200-700q0 25 17.5 42.5T260-640Zm220 160Z",
    );

    iconSvg.appendChild(path);
    categoryTag.appendChild(iconSvg);

    const textSpan = document.createElement("span");
    textSpan.textContent = this.formatTitle(this.title);
    categoryTag.appendChild(textSpan);

    categoryTag.addEventListener("click", () => {
      const categoryInput = document.getElementById("category");
      const categoryQidInput = document.getElementById("category_qid");

      if (categoryInput) {
        categoryInput.value = this.title;
      }

      if (categoryQidInput) {
        categoryQidInput.value = this.qid;
      }
    });

    this.shadowRoot.appendChild(categoryTag);
  }
}

customElements.define("wiki-category", WikiCategory);
