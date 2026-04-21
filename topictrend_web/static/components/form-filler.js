/**
 * <form-filler> web component
 *
 * Reads URL search parameters and populates the enclosing form's fields.
 * Each URL param is matched to a form field by name. For radio groups the
 * matching input[name=X][value=Y] is checked. For plain text inputs the
 * value is decoded (underscores replaced with spaces). After filling, a
 * "form-fill-complete" event is dispatched on the form so the page can
 * trigger submission.
 */
class FormFiller extends HTMLElement {
	connectedCallback() {
		this.style.display = "contents";
		const form = this.querySelector("form");
		if (!form) return;

		const params = new URLSearchParams(window.location.search);
		if (!params.size) return;

		let filled = false;

		for (const [name, value] of params) {
			const el = form.elements[name];
			if (!el) continue;

			// Radio group: form.elements[name] is a RadioNodeList
			if (el instanceof RadioNodeList) {
				const radio = form.querySelector(
					`input[name="${name}"][value="${value}"]`,
				);
				if (radio) {
					radio.checked = true;
					filled = true;
				}
				continue;
			}

			// Plain text inputs: decode underscores
			const decoded = el.type === "text" ? value.replaceAll("_", " ") : value;
			el.value = decoded;
			filled = true;
		}

		if (filled) {
			// Defer dispatch so synchronous DOMContentLoaded handlers in page JS
			// have a chance to register their form-fill-complete listeners first.
			setTimeout(
				() => form.dispatchEvent(new CustomEvent("form-fill-complete")),
				0,
			);
		}
	}
}

customElements.define("form-filler", FormFiller);
