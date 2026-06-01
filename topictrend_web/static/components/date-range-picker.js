/**
 * <date-range-picker> web component
 *
 * Wraps two native <input type="date"> children (data-role="start" / "end")
 * without replacing them: the inputs keep their id, name and YYYY-MM-DD value,
 * stay in form.elements, and remain readable via getElementById — so existing
 * page JS, form submission and the <form-filler> permalink path are untouched.
 *
 * On top of the (visually hidden) inputs it renders a combined trigger field
 * that opens a popover with a preset sidebar, a two-month range calendar and
 * Apply / Cancel. Apply writes the chosen dates back into the wrapped inputs
 * and dispatches "change"; it never submits the form.
 */

const STYLE_ID = "date-range-picker-styles";
const WEEKDAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
let instanceCount = 0;

function injectStyles() {
	if (document.getElementById(STYLE_ID)) return;
	const link = document.createElement("link");
	link.id = STYLE_ID;
	link.rel = "stylesheet";
	link.href = new URL("./date-range-picker.css", import.meta.url).href;
	document.head.appendChild(link);
}

// Local YYYY-MM-DD formatting. We deliberately avoid date-utils' formatDateToISO
// here because it goes through toISOString() (UTC) and would shift calendar dates
// built at local midnight by a day in non-UTC timezones.
function toISO(date) {
	const y = date.getFullYear();
	const m = String(date.getMonth() + 1).padStart(2, "0");
	const d = String(date.getDate()).padStart(2, "0");
	return `${y}-${m}-${d}`;
}

function parseISO(value) {
	const [y, m, d] = value.split("-").map(Number);
	if (!y || !m || !d) return null;
	return new Date(y, m - 1, d);
}

function startOfDay(date) {
	return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function startOfMonth(date) {
	return new Date(date.getFullYear(), date.getMonth(), 1);
}

function endOfMonth(date) {
	return new Date(date.getFullYear(), date.getMonth() + 1, 0);
}

function addMonths(date, n) {
	return new Date(date.getFullYear(), date.getMonth() + n, 1);
}

function addDays(date, n) {
	const r = new Date(date);
	r.setDate(r.getDate() + n);
	return r;
}

function isSameDay(a, b) {
	return (
		a &&
		b &&
		a.getFullYear() === b.getFullYear() &&
		a.getMonth() === b.getMonth() &&
		a.getDate() === b.getDate()
	);
}

function isSameMonth(a, b) {
	return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth();
}

function formatDisplay(date) {
	return date.toLocaleDateString();
}

class DateRangePicker extends HTMLElement {
	connectedCallback() {
		injectStyles();

		this.startInput = this.querySelector('input[data-role="start"]');
		this.endInput = this.querySelector('input[data-role="end"]');
		if (!this.startInput || !this.endInput) return;

		this.minDate = parseISO(this.getAttribute("min-date") || "2015-07-01");
		this.maxDate = startOfDay(new Date());
		this.id ||= `drp-${instanceCount++}`;
		this.panelId = `${this.id}-panel`;

		// Tentative (uncommitted) selection, plus the left month on display.
		this.selStart = null;
		this.selEnd = null;
		this.viewMonth = startOfMonth(this.maxDate);

		this.startInput.classList.add("drp-native");
		this.endInput.classList.add("drp-native");

		this.render();
		this.bindEvents();

		// Reflect whatever the page's own DOMContentLoaded handler sets as the
		// default values. setTimeout(0) runs after those synchronous handlers.
		setTimeout(() => this.syncDisplay(), 0);
		this.closest("form")?.addEventListener("form-fill-complete", () =>
			this.syncDisplay(),
		);
		this.startInput.addEventListener("change", () => this.syncDisplay());
		this.endInput.addEventListener("change", () => this.syncDisplay());
	}

	render() {
		const trigger = document.createElement("button");
		trigger.type = "button";
		trigger.className = "drp-trigger cdx-text-input__input";
		trigger.setAttribute("popovertarget", this.panelId);
		trigger.style.setProperty("anchor-name", `--${this.id}`);
		trigger.innerHTML = `
			<svg class="drp-icon" xmlns="http://www.w3.org/2000/svg" height="16" width="16"
				viewBox="0 -960 960 960" fill="currentColor" aria-hidden="true">
				<path d="M200-80q-33 0-56.5-23.5T120-160v-560q0-33 23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v560q0 33-23.5 56.5T760-80H200Zm0-80h560v-400H200v400Zm0-480h560v-80H200v80Z"/>
			</svg>
			<span class="drp-trigger-text">Select date range</span>`;

		const panel = document.createElement("div");
		panel.className = "drp-panel";
		panel.id = this.panelId;
		panel.setAttribute("popover", "");
		panel.style.setProperty("position-anchor", `--${this.id}`);
		panel.innerHTML = `
			<div class="drp-fields">
				<input class="drp-field cdx-text-input__input" data-field="start" readonly>
				<input class="drp-field cdx-text-input__input" data-field="end" readonly>
			</div>
			<div class="drp-body">
				<div class="drp-calendars"></div>
				<div class="drp-presets">
					<div class="drp-preset-list"></div>
					<div class="drp-actions">
						<button type="button" class="drp-apply cdx-button cdx-button--action-progressive cdx-button--weight-primary">Apply</button>
						<button type="button" class="drp-cancel cdx-button">Cancel</button>
					</div>
				</div>
			</div>`;

		this.append(trigger, panel);

		this.trigger = trigger;
		this.panel = panel;
		this.triggerText = trigger.querySelector(".drp-trigger-text");
		this.calendars = panel.querySelector(".drp-calendars");
		this.presetList = panel.querySelector(".drp-preset-list");
		this.startField = panel.querySelector('[data-field="start"]');
		this.endField = panel.querySelector('[data-field="end"]');
		this.applyBtn = panel.querySelector(".drp-apply");
		this.cancelBtn = panel.querySelector(".drp-cancel");

		this.renderPresets();
	}

	// Presets are recomputed relative to "today" each time the picker opens.
	buildPresets() {
		const today = this.maxDate;
		const lastMonthRef = addMonths(today, -1);
		return [
			{ label: "Last 7 days", start: addDays(today, -7), end: today },
			{ label: "Last 15 days", start: addDays(today, -15), end: today },
			{ label: "Last 30 days", start: addDays(today, -30), end: today },
			{ label: "Last 60 days", start: addDays(today, -60), end: today },
			{
				label: "Last 3 months",
				start: new Date(
					today.getFullYear(),
					today.getMonth() - 3,
					today.getDate(),
				),
				end: today,
			},
			{
				label: "Last month",
				start: startOfMonth(lastMonthRef),
				end: endOfMonth(lastMonthRef),
			},
			{
				label: "This year",
				start: new Date(today.getFullYear(), 0, 1),
				end: today,
			},
			{
				label: "Last year",
				start: new Date(today.getFullYear() - 1, 0, 1),
				end: new Date(today.getFullYear() - 1, 11, 31),
			},
		];
	}

	renderPresets() {
		this.presets = this.buildPresets();
		this.presetList.innerHTML = "";
		this.presets.forEach((preset, index) => {
			const btn = document.createElement("button");
			btn.type = "button";
			btn.className = "drp-preset";
			btn.dataset.preset = String(index);
			btn.textContent = preset.label;
			this.presetList.appendChild(btn);
		});
		const custom = document.createElement("button");
		custom.type = "button";
		custom.className = "drp-preset drp-preset--custom";
		custom.dataset.preset = "custom";
		custom.textContent = "Custom range";
		this.presetList.appendChild(custom);
	}

	bindEvents() {
		this.panel.addEventListener("toggle", (event) => {
			if (event.newState === "open") this.onOpen();
		});

		this.presetList.addEventListener("click", (event) => {
			const btn = event.target.closest(".drp-preset");
			if (!btn || btn.dataset.preset === "custom") return;
			const preset = this.presets[Number(btn.dataset.preset)];
			this.selStart = startOfDay(preset.start);
			this.selEnd = startOfDay(preset.end);
			this.viewMonth = startOfMonth(this.selStart);
			this.refresh();
		});

		this.calendars.addEventListener("click", (event) => {
			const nav = event.target.closest(".drp-nav");
			if (nav) {
				this.viewMonth = addMonths(this.viewMonth, Number(nav.dataset.step));
				this.renderCalendars();
				return;
			}
			const day = event.target.closest(".drp-day:not(.drp-day--disabled)");
			if (!day) return;
			this.selectDay(parseISO(day.dataset.date));
		});

		this.applyBtn.addEventListener("click", () => this.apply());
		this.cancelBtn.addEventListener("click", () => this.panel.hidePopover());
	}

	onOpen() {
		const start = parseISO(this.startInput.value);
		const end = parseISO(this.endInput.value);
		this.selStart = start ? startOfDay(start) : null;
		this.selEnd = end ? startOfDay(end) : null;
		this.viewMonth = startOfMonth(this.selStart || this.maxDate);
		this.renderPresets();
		this.refresh();
	}

	selectDay(day) {
		if (!this.selStart || this.selEnd) {
			// Start a fresh range.
			this.selStart = day;
			this.selEnd = null;
		} else if (day < this.selStart) {
			this.selStart = day;
		} else {
			this.selEnd = day;
		}
		this.refresh();
	}

	refresh() {
		this.renderCalendars();
		this.startField.value = this.selStart ? formatDisplay(this.selStart) : "";
		this.endField.value = this.selEnd ? formatDisplay(this.selEnd) : "";
		this.applyBtn.disabled = !(this.selStart && this.selEnd);
		this.highlightActivePreset();
	}

	highlightActivePreset() {
		let activeIndex = -1;
		if (this.selStart && this.selEnd) {
			activeIndex = this.presets.findIndex(
				(p) =>
					isSameDay(startOfDay(p.start), this.selStart) &&
					isSameDay(startOfDay(p.end), this.selEnd),
			);
		}
		for (const btn of this.presetList.querySelectorAll(".drp-preset")) {
			const isCustom = btn.dataset.preset === "custom";
			const isActive = isCustom
				? activeIndex === -1 && Boolean(this.selStart)
				: Number(btn.dataset.preset) === activeIndex;
			btn.classList.toggle("drp-preset--active", isActive);
		}
	}

	renderCalendars() {
		const left = this.viewMonth;
		const right = addMonths(left, 1);
		this.calendars.innerHTML =
			this.renderMonth(left, "left") + this.renderMonth(right, "right");
	}

	renderMonth(monthStart, side) {
		const year = monthStart.getFullYear();
		const month = monthStart.getMonth();
		const title = monthStart.toLocaleDateString(undefined, {
			month: "long",
			year: "numeric",
		});
		const prevDisabled = isSameMonth(monthStart, startOfMonth(this.minDate));
		const nextDisabled = isSameMonth(monthStart, startOfMonth(this.maxDate));

		const nav =
			side === "left"
				? `<button type="button" class="drp-nav" data-step="-1" aria-label="Previous month"${
						prevDisabled ? " disabled" : ""
					}>‹</button><span class="drp-month-title">${title}</span><span class="drp-nav-spacer"></span>`
				: `<span class="drp-nav-spacer"></span><span class="drp-month-title">${title}</span><button type="button" class="drp-nav" data-step="1" aria-label="Next month"${
						nextDisabled ? " disabled" : ""
					}>›</button>`;

		let cells = WEEKDAYS.map((d) => `<span class="drp-dow">${d}</span>`).join(
			"",
		);
		const firstWeekday = new Date(year, month, 1).getDay();
		const daysInMonth = new Date(year, month + 1, 0).getDate();
		for (let i = 0; i < firstWeekday; i++) {
			cells += `<span class="drp-day drp-day--empty"></span>`;
		}
		for (let d = 1; d <= daysInMonth; d++) {
			const date = new Date(year, month, d);
			const disabled = date < this.minDate || date > this.maxDate;
			const classes = ["drp-day"];
			if (disabled) classes.push("drp-day--disabled");
			if (isSameDay(date, this.selStart)) classes.push("drp-day--start");
			if (isSameDay(date, this.selEnd)) classes.push("drp-day--end");
			if (
				this.selStart &&
				this.selEnd &&
				date > this.selStart &&
				date < this.selEnd
			) {
				classes.push("drp-day--in-range");
			}
			cells += `<button type="button" class="${classes.join(
				" ",
			)}" data-date="${toISO(date)}"${disabled ? " disabled" : ""}>${d}</button>`;
		}

		return `<div class="drp-cal"><div class="drp-cal-header">${nav}</div><div class="drp-grid">${cells}</div></div>`;
	}

	apply() {
		if (!this.selStart || !this.selEnd) return;
		this.writeInput(this.startInput, toISO(this.selStart));
		this.writeInput(this.endInput, toISO(this.selEnd));
		this.syncDisplay();
		this.panel.hidePopover();
	}

	writeInput(input, value) {
		input.value = value;
		input.dispatchEvent(new Event("change", { bubbles: true }));
	}

	syncDisplay() {
		const start = parseISO(this.startInput.value);
		const end = parseISO(this.endInput.value);
		this.triggerText.textContent =
			start && end
				? `${formatDisplay(start)} – ${formatDisplay(end)}`
				: "Select date range";
	}
}

customElements.define("date-range-picker", DateRangePicker);
