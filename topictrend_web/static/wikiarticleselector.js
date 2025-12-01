import { autocomp } from "./autocomp.js";

class WikiArticleSelector extends HTMLElement {
  constructor() {
    super();
    this.languageSelector = null;
  }

  static get observedAttributes() {
    return ["value", "placeholder", "required", "wiki"];
  }

  connectedCallback() {
    this.render();
    this.setupAutocomplete();
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (this.input) {
      if (name === "value") {
        this.input.value = newValue || "";
      } else if (name === "placeholder") {
        this.input.placeholder = newValue || "";
      } else if (name === "required") {
        this.input.required = newValue !== null;
      } else if (name === "wiki" && oldValue !== newValue) {
        this.wiki = newValue;
      }
    }
  }

  render() {
    this.innerHTML = `
      <style>
        :host {
            display: inline-block;
        }

 
        wiki-article-selector {
          input {
      border: 0;
      }
          .autocomp {
              background: var(--background-color-base);
              border-radius: 0 0 5px 5px;
              border: 1px solid var(--border-color-base);
              border-top: 0;
              box-shadow: 2px 2px 2px #eee;
              text-align: left;
          }

          .autocomp-item {
              padding-bottom: 5px;
              padding: 10px;
              cursor: pointer;

              &:hover,
              &.autocomp-sel {
                background: var(--background-color-interactive--hover);
                font-weight: bold;
              }
          }
        }
      </style>
      <input 
        type="text" 
        class="title-input cdx-text-input__input"
        placeholder="${this.getAttribute("placeholder") || "Enter article title"}"
        value="${this.getAttribute("value") || ""}"
        ${this.getAttribute("required") !== null ? "required" : ""}
      />
    `;

    this.input = this.querySelector(".title-input");
  }

  setupAutocomplete() {
    if (!this.input) return;

    autocomp(this.input, {
      onQuery: async (val) => {
        const query = val.trim();
        if (query.length < 2) return [];

        const language = this.wiki || "en";
        return await this.searchWikipediaTitles(language, query);
      },
      onSelect: (result_item) => {
        return result_item;
      },
    });
  }

  async searchWikipediaTitles(language, query) {
    try {
      const response = await fetch(
        `https://${language}.wikipedia.org/w/api.php?action=query&list=prefixsearch&psprofile=fuzzy&pssearch=${encodeURIComponent(query)}&limit=10&origin=*&format=json`,
      );

      if (!response.ok) {
        return [];
      }

      const data = await response.json();
      return data.query.prefixsearch.map((page) => page.title);
    } catch (error) {
      console.error("Error searching Wikipedia titles:", error);
      return [];
    }
  }

  // Public API
  get value() {
    return this.input ? this.input.value : this.getAttribute("value") || "";
  }

  set value(val) {
    if (this.input) {
      this.input.value = val;
    }
    this.setAttribute("value", val);
  }

  focus() {
    if (this.input) {
      this.input.focus();
    }
  }

  blur() {
    if (this.input) {
      this.input.blur();
    }
  }
}

customElements.define("wiki-article-selector", WikiArticleSelector);

export { WikiArticleSelector };
