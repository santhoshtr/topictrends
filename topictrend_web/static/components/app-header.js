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
		const pageType = this.getAttribute("page-type") || "home";
		const titleMap = {
			home: "Topic Trends",
			pageviews: "Topic Trends - Pageviews",
			pageedits: "Topic Trends - Page Edits",
			search: "Topic Trends - Search",
		};
		return titleMap[pageType] || "Topic Trends";
	}

	getActiveLink() {
		const path = window.location.pathname;
		if (path === "/" || path === "/index.html") {
			return "/";
		}
		if (path === "/pageviews/trends") {
			return "/pageviews/trends";
		}
		if (path === "/pageviews/delta") {
			return "/pageviews/delta";
		}
		if (path === "/pageedits/trends") {
			return "/pageedits/trends";
		}
		if (path === "/pageedits/delta") {
			return "/pageedits/delta";
		}
		if (path === "/search") {
			return "/search";
		}
		return "/";
	}

	render() {
		const title = this.getTitle();
		const activePath = this.getActiveLink();

		const navLinks = [
			{ label: "Home", href: "/" },
			{ label: "Pageview Trends", href: "/pageviews/trends" },
			{ label: "Pageview Delta", href: "/pageviews/delta" },
			{ label: "Page Edit Trends", href: "/pageedits/trends" },
			{ label: "Page Edit Delta", href: "/pageedits/delta" },
			{ label: "Search", href: "/search" },
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
