import { autocomp } from "./autocomp.js";

document.addEventListener("DOMContentLoaded", function () {
  document.getElementById("trend-form").addEventListener("submit", onSubmit);

  // Set up wiki selector change handler
  const wikiSelector = document.getElementById("wiki");
  const articleElement = document.getElementById("article");
  const categoryElement = document.getElementById("category");

  wikiSelector.addEventListener("change", function () {
    const wikiValue = this.value.replaceAll("wiki", "");
    articleElement?.setAttribute("wiki", wikiValue);
    categoryElement?.setAttribute("wiki", wikiValue);
  });

  // Initialize with current wiki value
  const wikiValue = wikiSelector.value.replaceAll("wiki", "");

  articleElement.setAttribute("wiki", wikiValue);
  categoryElement.setAttribute("wiki", wikiValue);

  initializeChart();
  populateFormFromQueryParams();
});

async function onSubmit(event) {
  event.preventDefault();

  const params = new URLSearchParams();
  const type = document.querySelector('input[name="type"]:checked').value;
  const wiki = document.getElementById("wiki").value;
  const startDate = document.getElementById("start_date").value;
  const endDate = document.getElementById("end_date").value;
  const depth = 2;

  params.append("type", type);
  params.append("wiki", wiki);
  params.append("start_date", startDate);
  params.append("end_date", endDate);
  params.append("depth", depth);
  try {
    if (type === "category") {
      const category = document
        .getElementById("category")
        .value.replaceAll(" ", "_");
      params.append("category", category);

      // Update the browser URL with the new parameters
      const newUrl = `${window.location.pathname}?${params.toString()}`;
      window.history.pushState({}, "", newUrl);

      await fetchCategoryPageviews(wiki, category, startDate, endDate, depth);
      await renderSubCategories(wiki, category);
    } else if (type === "article") {
      const article = document
        .getElementById("article")
        .value.replaceAll(" ", "_");
      params.append("article", article);

      // Update the browser URL with the new parameters
      const newUrl = `${window.location.pathname}?${params.toString()}`;
      window.history.pushState({}, "", newUrl);
      await fetchArticlePageviews(wiki, article, startDate, endDate);
    }
  } catch (error) {
    console.error("Error:", error);
    showMessage("Failed to fetch data. Please try again.", "error");
  }
}

let chartInstance = null;

function initializeChart() {
  const theme = window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";

  chartInstance = echarts.init(document.getElementById("chart"), theme, {
    renderer: "svg",
  });

  const initialOption = {
    darkMode: "auto",
    color: [
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
    ],
    title: {
      text: "Pageviews Trend",
    },
    tooltip: {
      trigger: "axis",
    },
    legend: {
      top: "bottom",
      left: "center", // Center the legend horizontally
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
  window.onresize = chartInstance.resize;
}

function updateChart(data, label) {
  const existingOption = chartInstance.getOption();

  // Update xAxis data if new dates are present
  const newDates = data.map((item) => item.date);
  const existingDates = existingOption.xAxis[0].data;
  const mergedDates = Array.from(new Set([...existingDates, ...newDates]));
  mergedDates.sort(); // Ensure dates are sorted
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

async function renderSubCategories(wiki, category) {
  const categoryListContainer = document.getElementById("category-list");
  const apiUrl = `/api/list/sub_categories?wiki=${wiki}&category=${category}`;

  const response = await fetch(apiUrl);
  const subcategoryIds = await response.json();
  if (!response.ok) {
    throw new Error("Failed to fetch data");
  }

  categoryListContainer.innerHTML = ""; // Clear previous results

  const subheading = document.createElement("h3");
  subheading.textContent = "Subcategories";
  categoryListContainer.appendChild(subheading);

  const ul = document.createElement("ul");
  const subcategories = await getTitlesFromIds(subcategoryIds, wiki);

  subcategories.forEach((title, id) => {
    title = title.replace(/^.*:\s*/, "");
    const li = document.createElement("li");
    li.id = id;
    const categoryLabel = document.createElement("span");
    categoryLabel.href = "#";
    categoryLabel.textContent = title.replaceAll("_", " ");
    const plotButton = document.createElement("button");
    plotButton.title = "Plot pageviews for this category";
    plotButton.className = "plot-button";
    plotButton.innerHTML = `
      <svg xmlns="http://www.w3.org/2000/svg" 
        height="16px" viewBox="0 -960 960 960"
        width="16px" fill="currentColor">
      <path d="m140-220-60-60 300-300 160 160 284-320 56 56-340 384-160-160-240 240Z"/>
      </svg>
      `;
    plotButton.addEventListener("click", (event) => {
      event.preventDefault();
      const startDate = document.getElementById("start_date").value;
      const endDate = document.getElementById("end_date").value;
      const depth = 2;

      fetchCategoryPageviews(wiki, title, startDate, endDate, depth);
    });
    const analyseButton = document.createElement("button");
    analyseButton.title = "Analyse this category";
    analyseButton.className = "analyse-button";
    analyseButton.innerHTML = `
    <svg xmlns="http://www.w3.org/2000/svg" 
    height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor">
    <path d="M400-320q100 0 170-70t70-170q0-100-70-170t-170-70q-100 0-170 70t-70 170q0 100 70 170t170 70Zm-40-120v-280h80v280h-80Zm-140 0v-200h80v200h-80Zm280 0v-160h80v160h-80ZM824-80 597-307q-41 32-91 49.5T400-240q-134 0-227-93T80-560q0-134 93-227t227-93q134 0 227 93t93 227q0 56-17.5 106T653-363l227 227-56 56Z"/></svg>
    `;

    analyseButton.addEventListener("click", (event) => {
      event.preventDefault();
      const urlParams = new URLSearchParams(window.location.search);
      urlParams.set("category", title);
      urlParams.set("type", "category");
      const newUrl = `${window.location.pathname}?${urlParams.toString()}`;
      window.location.href = newUrl;
    });
    li.appendChild(categoryLabel);
    li.appendChild(plotButton);
    li.appendChild(analyseButton);
    ul.appendChild(li);
  });

  categoryListContainer.appendChild(ul);
}

async function getTitlesFromIds(ids, wikicode) {
  const pageIds = ids.join("|");
  const apiUrl = `https://${wikicode.replaceAll("wiki", "")}.wikipedia.org/w/api.php?action=query&prop=info&pageids=${pageIds}&format=json&formatversion=2&origin=*`;

  try {
    const response = await fetch(apiUrl, {
      headers: {
        "User-Agent": "TopicTrend/1.0 (https://topictrends.wmcloud.org)",
      },
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const data = await response.json();
    const titlesMap = new Map();

    if (data.query && data.query.pages) {
      data.query.pages.forEach((page) => {
        titlesMap.set(page.pageid, page.title);
      });
    }

    return titlesMap;
  } catch (error) {
    console.error("Error fetching titles:", error);
    return new Map();
  }
}

document.addEventListener("DOMContentLoaded", function () {
  const startDatePicker = document.getElementById("start_date");
  const endDatePicker = document.getElementById("end_date");
  const today = new Date();

  // Format the date to "YYYY-MM-DD" as required by the input type="date"
  let year = today.getFullYear();
  let month = String(today.getMonth() + 1).padStart(2, "0"); // Months are 0-indexed
  let day = String(today.getDate()).padStart(2, "0");
  endDatePicker.value = `${year}-${month}-${day}`;

  const oneMonthAgo = new Date(
    today.getFullYear(),
    today.getMonth() - 1,
    today.getDate(),
  );
  year = oneMonthAgo.getFullYear();
  month = String(oneMonthAgo.getMonth() + 1).padStart(2, "0"); // Months are 0-indexed
  day = String(oneMonthAgo.getDate()).padStart(2, "0");

  startDatePicker.value = `${year}-${month}-${day}`;
});

function showMessage(message, type) {
  const messageEl = document.getElementById("status");
  messageEl.classList.remove("error-message");
  messageEl.classList.remove("success-message");
  messageEl.classList.add(
    type === "error" ? "error-message" : "success-message",
  );
  messageEl.textContent = message;
}

async function fetchCategoryPageviews(
  wiki,
  category,
  startDate,
  endDate,
  depth,
) {
  const apiUrl = `/api/pageviews/category?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&depth=${depth}&category=${encodeURIComponent(
    category,
  )}`;
  const label = `Category: ${wiki} - ${category.replaceAll("_", " ")}`;

  try {
    const startTime = performance.now();
    const response = await fetch(apiUrl);
    if (!response.ok) {
      throw new Error("Failed to fetch data");
    }

    const data = await response.json();
    updateChart(data, label);
    const endTime = performance.now();
    const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
    showMessage(`Fetched ${label} in ${timeTaken} seconds.`, "success");
  } catch (error) {
    console.error("Error:", error);
    showMessage("Failed to fetch category data. Please try again.", "error");
  }
}

async function fetchArticlePageviews(wiki, article, startDate, endDate) {
  const apiUrl = `/api/pageviews/article?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&article=${encodeURIComponent(
    article,
  )}`;
  const label = `Article: ${wiki} - ${article.replaceAll("_", " ")}`;

  try {
    const startTime = performance.now();
    const response = await fetch(apiUrl);
    if (!response.ok) {
      throw new Error("Failed to fetch data");
    }

    const data = await response.json();
    updateChart(data, label);
    const endTime = performance.now();
    const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
    showMessage(`Fetched ${label} in ${timeTaken} seconds.`, "success");
  } catch (error) {
    console.error("Error:", error);
    showMessage("Failed to fetch article data. Please try again.", "error");
  }
}
function populateFormFromQueryParams() {
  const urlParams = new URLSearchParams(window.location.search);

  const type = urlParams.get("type");
  const wiki = urlParams.get("wiki");
  const startDate = urlParams.get("start_date");
  const endDate = urlParams.get("end_date");
  const category = urlParams.get("category");
  const article = urlParams.get("article");

  if (type) {
    document.querySelector(`input[name="type"][value="${type}"]`).checked =
      true;
  }
  if (wiki) {
    document.getElementById("wiki").value = wiki;
  }
  if (startDate) {
    document.getElementById("start_date").value = startDate;
  }
  if (endDate) {
    document.getElementById("end_date").value = endDate;
  }
  if (type === "category" && category) {
    document.getElementById("category").value = category.replaceAll("_", " ");
  }
  if (type === "article" && article) {
    document.getElementById("article").value = article.replaceAll("_", " ");
  }

  if (type && wiki && startDate && endDate) {
    onSubmit(new Event("submit"));
  }
}
