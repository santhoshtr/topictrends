/**
 * Date formatting and manipulation utilities
 */

/**
 * Format a Date object to ISO date string (YYYY-MM-DD)
 * @param {Date} date - Date object to format
 * @returns {string} Formatted date string
 */
export function formatDateToISO(date) {
	return date.toISOString().split("T")[0];
}

/**
 * Format a Date object to API format (YYYYMMDD)
 * @param {Date} date - Date object to format
 * @returns {string} Formatted date string without hyphens
 */
export function formatDateForAPI(date) {
	return date.toISOString().split("T")[0].replace(/-/g, "");
}

/**
 * Get date N days ago from today
 * @param {number} days - Number of days to subtract
 * @returns {Date} Date object
 */
export function getDaysAgo(days) {
	const date = new Date();
	date.setDate(date.getDate() - days);
	return date;
}

/**
 * Get date N months ago from today
 * @param {number} months - Number of months to subtract
 * @returns {Date} Date object
 */
export function getMonthsAgo(months) {
	const date = new Date();
	date.setMonth(date.getMonth() - months);
	return date;
}

/**
 * Get yesterday's date
 * @returns {Date} Yesterday's date
 */
export function getYesterday() {
	return getDaysAgo(1);
}

/**
 * Parse ISO date string (YYYY-MM-DD) to Date object
 * @param {string} dateString - ISO date string
 * @returns {Date} Date object
 */
export function parseISODate(dateString) {
	return new Date(dateString);
}
