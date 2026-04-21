class WikiSelector extends HTMLElement {
	static get observedAttributes() {
		return ["wiki", "name", "form"];
	}

	connectedCallback() {
		this.render();
		this.populate();
	}

	attributeChangedCallback(name, oldValue, newValue) {
		if (oldValue === newValue) return;
		if (name === "wiki") {
			const select = this.querySelector("select");
			if (select) select.value = newValue;
		} else if (name === "name") {
			const select = this.querySelector("select");
			if (select) select.name = newValue || "wiki";
		} else if (name === "form") {
			const select = this.querySelector("select");
			if (select) {
				if (newValue) {
					select.setAttribute("form", newValue);
				} else {
					select.removeAttribute("form");
				}
			}
		}
	}

	get value() {
		return this.querySelector("select")?.value ?? "";
	}

	set value(v) {
		const select = this.querySelector("select");
		if (select) select.value = v;
	}

	render() {
		const formAttr = this.getAttribute("form");
		const nameAttr = this.getAttribute("name") || "wiki";

		this.innerHTML = `
<div class="form-item">
  <label for="wiki">Wiki</label>
  <select id="wiki" name="${nameAttr}" class="cdx-select"${formAttr ? ` form="${formAttr}"` : ""}>
    <option value="enwiki">en - English</option>
  </select>
</div>`;
	}

	async populate() {
		const select = this.querySelector("select");
		const defaultWiki = this.getAttribute("wiki") || "enwiki";

		try {
			const response = await fetch("/static/wikis.json");
			if (!response.ok) throw new Error(`HTTP ${response.status}`);
			const wikis = await response.json();

			select.innerHTML = "";
			for (const wiki of wikis) {
				const option = document.createElement("option");
				option.value = wiki.code;
				option.textContent = `${wiki.langcode} - ${wiki.name}`;
				if (wiki.code === defaultWiki) option.selected = true;
				select.appendChild(option);
			}
		} catch (error) {
			console.error("Failed to load wiki list:", error);
		}
	}
}

customElements.define("wiki-selector", WikiSelector);
