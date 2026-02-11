class AppHeader extends HTMLElement {
	static get observedAttributes() {
		return ["page-type"];
	}

	constructor() {
		super();
		this.attachShadow({ mode: "open" });
	}

	connectedCallback() {
		this.render();
	}

	attributeChangedCallback() {
		this.render();
	}

	getTitle() {
		const pageType = this.getAttribute("page-type") || "pageviews";
		const titleMap = {
			pageviews: "Topic Trends - Pageviews",
			pageedits: "Topic Trends - Page Edits",
			search: "Topic Trends - Search",
			about: "Topic Trends - About",
		};
		return titleMap[pageType] || "Topic Trends";
	}

	getActiveLink() {
		const path = window.location.pathname;
		if (path === "/" || path === "/index.html") {
			return "/";
		}
		if (path.startsWith("/pageviews")) {
			return "/pageviews/delta";
		}
		if (path.startsWith("/pageedits")) {
			return "/pageedits/delta";
		}
		if (path.startsWith("/search")) {
			return "/search";
		}
		if (path.startsWith("/about")) {
			return "/about";
		}
		return "/";
	}

	render() {
		const title = this.getTitle();
		const activePath = this.getActiveLink();

		const navLinks = [
			{ label: "Page views", href: "/" },
			{ label: "Page edits", href: "/pageedits/delta" },
			{ label: "Search Categories", href: "/search" },
			{ label: "About", href: "/about" },
		];

		const navHtml = navLinks
			.map(
				(link) => `
			<li>
				<a 
					href="${link.href}" 
					class="nav-link ${link.href === activePath ? "active" : ""}"
				>
					${link.label}
				</a>
			</li>
		`,
			)
			.join("");

		this.shadowRoot.innerHTML = `
			<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@wikimedia/codex-design-tokens/theme-wikimedia-ui.css">
			<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@wikimedia/codex-design-tokens/theme-wikimedia-ui-mode-dark.css" media="(prefers-color-scheme: dark)">
			<link rel="stylesheet" href="/static/components/app-header.css">
			
			<header class="header">
				<h1 class="header-title">${title}</h1>
				<nav>
					<ul class="header-nav">
						${navHtml}
					</ul>
				</nav>
			</header>
		`;
	}
}

customElements.define("app-header", AppHeader);
