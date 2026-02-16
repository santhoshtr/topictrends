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

	render() {
		const title = this.getTitle();
		const path = window.location.pathname;

		// Determine active states
		const isPageviewsActive = path.startsWith("/pageviews/");
		const isPageeditsActive = path.startsWith("/pageedits/");
		const isSearchActive = path === "/search";
		const isTrendsPageviews = path === "/pageviews/trends";
		const isDeltaPageviews = path === "/pageviews/delta";
		const isTrendsPageedits = path === "/pageedits/trends";
		const isDeltaPageedits = path === "/pageedits/delta";

		this.shadowRoot.innerHTML = `
			<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@wikimedia/codex-design-tokens/theme-wikimedia-ui.css">
			<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@wikimedia/codex-design-tokens/theme-wikimedia-ui-mode-dark.css" media="(prefers-color-scheme: dark)">
			<link rel="stylesheet" href="/static/components/app-header.css">
			
			<header class="header">
				<h1 class="header-title">${title}</h1>
				<nav>
					<ul class="header-nav">
						<li>
							<button 
								popovertarget="pageviews-menu" 
								class="nav-button ${isPageviewsActive ? "active" : ""}"
								id="pageviews-anchor"
							>
								Page views <span class="chevron">                                     <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" aria-hidden="true" width="16" height="16" ><path d="M17,9.17a1,1,0,0,0-1.41,0L12,12.71,8.46,9.17a1,1,0,0,0-1.41,0,1,1,0,0,0,0,1.42l4.24,4.24a1,1,0,0,0,1.42,0L17,10.59A1,1,0,0,0,17,9.17Z" fill="currentColor"></path></svg></span>
							</button>
							<div popover id="pageviews-menu" class="nav-menu" anchor="pageviews-anchor">
								<a href="/pageviews/trends" class="${isTrendsPageviews ? "active" : ""}">Trends</a>
								<a href="/pageviews/delta" class="${isDeltaPageviews ? "active" : ""}">Delta</a>
							</div>
						</li>
						<li>
							<button 
								popovertarget="pageedits-menu" 
								class="nav-button ${isPageeditsActive ? "active" : ""}"
								id="pageedits-anchor"
							>
								Page edits <span class="chevron">
                                      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" aria-hidden="true" width="16" height="16" ><path d="M17,9.17a1,1,0,0,0-1.41,0L12,12.71,8.46,9.17a1,1,0,0,0-1.41,0,1,1,0,0,0,0,1.42l4.24,4.24a1,1,0,0,0,1.42,0L17,10.59A1,1,0,0,0,17,9.17Z" fill="currentColor"></path></svg></span>
							</button>
							<div popover id="pageedits-menu" class="nav-menu" anchor="pageedits-anchor">
								<a href="/pageedits/trends" class="${isTrendsPageedits ? "active" : ""}">Trends</a>
								<a href="/pageedits/delta" class="${isDeltaPageedits ? "active" : ""}">Delta</a>
							</div>
						</li>
						<li>
							<a href="/search" class="nav-link ${isSearchActive ? "active" : ""}">Search</a>
						</li>
					</ul>
				</nav>
			</header>
		`;
	}
}

customElements.define("app-header", AppHeader);
