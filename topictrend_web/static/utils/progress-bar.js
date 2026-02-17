/**
 * Simple global progress bar utilities
 */

const progressBar = document.createElement("progress");
progressBar.id = "global-progress";
progressBar.setAttribute("data-visible", "false");
document.body.appendChild(progressBar);

/**
 * Show the progress bar
 */
export function showProgress() {
	progressBar.setAttribute("data-visible", "true");
}

/**
 * Hide the progress bar
 */
export function hideProgress() {
	progressBar.setAttribute("data-visible", "false");
}
