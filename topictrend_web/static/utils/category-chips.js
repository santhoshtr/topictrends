// Renders a list of category chips (a heading + <wiki-category> chips with a plot
// button) into `container`. Shared by the "Matched categories" list produced by topic
// search and the "Subcategories" list shown after a category analysis.
//
// items: [{ qid, title, title_en }] — `title` is localized to `wiki`.
// onPlot(qid, title): invoked when a chip's plot button is clicked.
const PLOT_ICON = `
  <svg xmlns="http://www.w3.org/2000/svg"
    height="16px" viewBox="0 -960 960 960"
    width="16px" fill="currentColor">
  <path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/>
  </svg>
  `;

// Relaxed semantic search for categories matching a plain-text topic. Returns items
// shaped for renderCategoryChips ({ qid, title, title_en }), `title` localized to `wiki`.
export async function searchCategories(
	wiki,
	topicText,
	{ matchThreshold = 0.5, limit = 10 } = {},
) {
	const query = topicText.replaceAll("_", " ").trim();
	const url = `/api/search/categories?query=${encodeURIComponent(
		query,
	)}&wiki=${wiki}&match_threshold=${matchThreshold}&limit=${limit}`;
	const response = await fetch(url);
	if (!response.ok) {
		throw new Error("Failed to search categories");
	}
	const data = await response.json();
	return (data.categories || []).map((c) => ({
		qid: c.category_qid,
		title: c.category_title || c.category_title_en,
		title_en: c.category_title_en,
	}));
}

export function renderCategoryChips(
	container,
	{ heading, items, wiki, onPlot },
) {
	container.innerHTML = "";
	if (!items || items.length === 0) {
		return;
	}

	const subheading = document.createElement("h3");
	subheading.textContent = heading;
	container.appendChild(subheading);

	const ul = document.createElement("ul");
	for (const { qid, title, title_en } of items) {
		const li = document.createElement("li");
		li.id = qid;

		const wikiCategory = document.createElement("wiki-category");
		wikiCategory.setAttribute("title", title);
		if (title_en && title_en !== title) {
			wikiCategory.setAttribute("title-en", title_en);
		}
		wikiCategory.setAttribute("qid", qid);
		wikiCategory.setAttribute("views", "0");
		if (wiki) {
			wikiCategory.setAttribute("wiki", wiki);
		}

		const plotButton = document.createElement("button");
		plotButton.title = "Plot this category";
		plotButton.className = "plot-button";
		plotButton.innerHTML = PLOT_ICON;
		plotButton.addEventListener("click", (event) => {
			event.preventDefault();
			onPlot(qid, title);
		});

		li.appendChild(wikiCategory);
		li.appendChild(plotButton);
		ul.appendChild(li);
	}

	container.appendChild(ul);
}
