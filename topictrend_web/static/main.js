import { autocomp } from "./autocomp.js";

document.addEventListener("DOMContentLoaded", async function () {
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

  await populateWikiDropdown();
  populateFormFromQueryParams();
});

async function onSubmit(event) {
  event.preventDefault();

  const params = new URLSearchParams();
  const type = document.querySelector('input[name="type"]:checked').value;
  const wiki = document.getElementById("wiki").value;
  const startDate = document.getElementById("start_date").value;
  const endDate = document.getElementById("end_date").value;
  const category_qid = document.getElementById("category_qid").value;
  const article_qid = document.getElementById("article_qid").value;
  const depth = document.getElementById("depth").value;

  params.append("type", type);
  params.append("wiki", wiki);
  params.append("start_date", startDate);
  params.append("end_date", endDate);
  params.append("depth", depth);
  if (category_qid) {
    params.append("category_qid", category_qid);
  }
  if (article_qid) {
    params.append("article_qid", article_qid);
  }
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
      await renderSubCategories(wiki, category, depth);
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
  document.getElementById("chart").style.display = "block";
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
  if (!chartInstance) {
    initializeChart();
  }

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

async function renderSubCategories(wiki, category, depth = 20) {
  const categoryListContainer = document.getElementById("category-list");
  const apiUrl = `/api/list/sub_categories?wiki=${wiki}&category=${category}`;

  const response = await fetch(apiUrl);
  const subcategories = await response.json();
  if (!response.ok) {
    throw new Error("Failed to fetch data");
  }

  categoryListContainer.innerHTML = ""; // Clear previous results

  const subheading = document.createElement("h3");
  subheading.textContent = "Subcategories";
  categoryListContainer.appendChild(subheading);

  const ul = document.createElement("ul");
  Object.entries(subcategories).forEach(([qid, title]) => {
    const li = document.createElement("li");
    li.id = qid;

    const wikiCategory = document.createElement("wiki-category");
    wikiCategory.setAttribute("title", title);
    wikiCategory.setAttribute("qid", qid);
    wikiCategory.setAttribute("views", "0");

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

      fetchCategoryPageviews(wiki, title, startDate, endDate, depth);
    });

    li.appendChild(wikiCategory);
    li.appendChild(plotButton);
    ul.appendChild(li);
  });

  categoryListContainer.appendChild(ul);
}

function renderTopArticles(wiki, topArticles) {
  const container = document.getElementById("top-articles");

  if (!topArticles || topArticles.length === 0) {
    return;
  }
  container.innerHTML = "";

  const subheading = document.createElement("h3");
  subheading.textContent = "Top Articles in Category";
  container.appendChild(subheading);
  topArticles.forEach((article) => {
    const articleEl = document.createElement("wiki-article-pageviews");
    articleEl.setAttribute("wiki", wiki);
    articleEl.setAttribute("title", article.title);
    articleEl.setAttribute("views", article.views.toString());
    articleEl.setAttribute("qid", article.qid.toString());
    articleEl.setAttribute("categories", "[]");
    container.appendChild(articleEl);
  });
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
    updateChart(data.views, label);
    const endTime = performance.now();
    const timeTaken = ((endTime - startTime) / 1000).toFixed(2);
    showMessage(`Fetched ${label} in ${timeTaken} seconds.`, "success");

    if (data.top_articles && data.top_articles.length > 0) {
      renderTopArticles(wiki, data.top_articles);
    }
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
    updateChart(data.views, label);
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
  const category_qid = urlParams.get("category_qid");
  const article_qid = urlParams.get("article_qid");
  const depth = urlParams.get("depth");

  if (type) {
    document.querySelector(`input[name="type"][value="${type}"]`).checked =
      true;
  }
  if (depth) {
    document.getElementById("depth").value = depth;
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
    if (category_qid) {
      document.getElementById("category_qid").value = category_qid;
    }
  }
  if (type === "article" && article) {
    document.getElementById("article").value = article.replaceAll("_", " ");
    if (article_qid) {
      document.getElementById("article_qid").value = article_qid;
    }
  }

  if (type && wiki && startDate && endDate) {
    onSubmit(new Event("submit"));
  }
}

async function populateWikiDropdown() {
  try {
    const response = await fetch("/static/wikis.json");
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const wikis = await response.json();
    const wikiSelect = document.getElementById("wiki");

    // Clear existing options
    wikiSelect.innerHTML = "";

    // Add options to dropdown
    wikis.forEach((wiki) => {
      const option = document.createElement("option");
      option.value = wiki.code;
      const displayName = `${wiki.langcode} - ${wiki.name}`;
      option.textContent = displayName;
      wikiSelect.appendChild(option);
    });

    console.log(`Loaded ${wikis.length} wikis to dropdown`);
  } catch (error) {
    console.error("Failed to load wiki list:", error);
    // Fallback to current hardcoded options
    console.log("📋 Using fallback wiki list");
  }
}

// Setup controls
document.addEventListener("DOMContentLoaded", () => {
  const loadButton = document.getElementById("wikitrends-btn");

  loadButton.addEventListener("click", () => {
    let topicTrends = document.querySelector("wiki-trends");

    const selectedWiki = wiki.value;
    if (!topicTrends) {
      let topicTrendsEl = document.createElement("wiki-trends");
      document.querySelector(".main").appendChild(topicTrendsEl);
      topicTrends = document.querySelector("wiki-trends");
    }

    topicTrends.setAttribute("wiki", selectedWiki);
    loadButton.disabled = true;

    // Re-enable button after a short delay to prevent rapid clicking
    setTimeout(() => {
      loadButton.disabled = false;
    }, 1000);
  });
});
