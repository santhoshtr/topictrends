// Landing page: reflect the real host in the MCP endpoint and wire copy buttons.

const endpoint = `${window.location.origin}/mcp`;

const urlEl = document.getElementById("mcp-url");
if (urlEl) {
	urlEl.textContent = endpoint;
}

const snippetEl = document.getElementById("mcp-config-snippet");
if (snippetEl) {
	snippetEl.textContent = JSON.stringify(
		{ mcpServers: { topictrends: { url: endpoint } } },
		null,
		2,
	);
}

for (const button of document.querySelectorAll(".mcp-copy")) {
	button.addEventListener("click", async () => {
		const target = document.getElementById(button.dataset.copyTarget);
		if (!target) {
			return;
		}
		await navigator.clipboard.writeText(target.textContent.trim());
		const original = button.textContent;
		button.textContent = "Copied";
		setTimeout(() => {
			button.textContent = original;
		}, 1500);
	});
}
