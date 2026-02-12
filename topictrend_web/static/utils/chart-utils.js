/**
 * ECharts initialization and update utilities
 */

import {
	CHART_GRID_BOTTOM,
	CHART_GRID_CONTAINLABEL,
	CHART_GRID_LEFT,
	CHART_GRID_RIGHT,
} from "./constants.js";

/**
 * Default color palette for charts
 */
export const DEFAULT_CHART_COLORS = [
	"#4b77d6",
	"#eeb533",
	"#fd7865",
	"#80cdb3",
	"#269f4b",
	"#b0c1f0",
	"#9182c2",
	"#d9b4cd",
	"#b0832b",
	"#a2a9b1",
];

/**
 * Initialize an ECharts instance with default configuration
 * @param {HTMLElement} chartElement - DOM element to render chart
 * @param {string} title - Chart title
 * @returns {Object} ECharts instance
 */
export function initializeChart(chartElement, title = "Pageviews Trend") {
	const theme = window.matchMedia("(prefers-color-scheme: dark)").matches
		? "dark"
		: "light";

	const chartInstance = echarts.init(chartElement, theme, {
		renderer: "svg",
	});

	const initialOption = {
		darkMode: "auto",
		color: DEFAULT_CHART_COLORS,
		title: {
			text: title,
		},
		tooltip: {
			trigger: "axis",
		},
		legend: {
			top: "bottom",
			left: "center",
		},
		xAxis: {
			type: "category",
			data: [],
		},
		yAxis: {
			type: "value",
		},
		series: [],
		toolbox: {
			show: true,
			feature: {
				dataZoom: {
					yAxisIndex: "none",
				},
				dataView: { readOnly: false },
				magicType: { type: ["line", "bar"] },
				restore: {},
				saveAsImage: {},
			},
		},
	};

	chartInstance.setOption(initialOption);
	window.onresize = () => chartInstance.resize();

	return chartInstance;
}

/**
 * Update chart with new data series
 * @param {Object} chartInstance - ECharts instance
 * @param {Array} data - Data array with {date, views} objects
 * @param {string} label - Series label
 */
export function updateChart(chartInstance, data, label) {
	const existingOption = chartInstance.getOption();

	// Update xAxis data if new dates are present
	const newDates = data.map((item) => item.date);
	const existingDates = existingOption.xAxis[0].data;
	const mergedDates = Array.from(new Set([...existingDates, ...newDates]));
	mergedDates.sort();

	chartInstance.setOption({
		xAxis: {
			data: mergedDates,
		},
	});

	// Add a new series for the new data
	chartInstance.setOption({
		series: [
			...existingOption.series,
			{
				name: label,
				data: data.map((item) => item.views),
				type: "line",
				smooth: true,
			},
		],
	});
}

/**
 * Clear all data from chart
 * @param {Object} chartInstance - ECharts instance
 */
export function clearChart(chartInstance) {
	chartInstance.setOption({
		xAxis: { data: [] },
		series: [],
	});
}
