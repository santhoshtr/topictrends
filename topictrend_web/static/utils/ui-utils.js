/**
 * UI feedback and interaction utilities
 */

import { MESSAGE_DISPLAY_DURATION_MS } from "./constants.js";

/**
 * Display a status message to the user
 * @param {string} message - Message text to display
 * @param {string} type - Message type ('error' or 'success')
 */
export function showMessage(message, type = "success") {
	const messageEl = document.getElementById("status");
	if (!messageEl) {
		console.warn("Status message element not found");
		return;
	}

	messageEl.classList.remove("error-message", "success-message");
	messageEl.classList.add(
		type === "error" ? "error-message" : "success-message",
	);
	messageEl.textContent = message;
}

/**
 * Clear the status message
 */
export function clearMessage() {
	const messageEl = document.getElementById("status");
	if (messageEl) {
		messageEl.textContent = "";
		messageEl.classList.remove("error-message", "success-message");
	}
}

/**
 * Show a temporary message that auto-clears
 * @param {string} message - Message text to display
 * @param {string} type - Message type ('error' or 'success')
 * @param {number} duration - Duration in milliseconds before auto-clear
 */
export function showTemporaryMessage(
	message,
	type = "success",
	duration = MESSAGE_DISPLAY_DURATION_MS,
) {
	showMessage(message, type);
	setTimeout(clearMessage, duration);
}
