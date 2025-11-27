document.addEventListener("DOMContentLoaded", function () {
  initializeChart();

  document
    .getElementById("trend-form")
    .addEventListener("submit", async (event) => {
      event.preventDefault();

      const type = document.querySelector('input[name="type"]:checked').value;
      const wiki = document.getElementById("wiki").value;
      const startDate = document.getElementById("start_date").value;
      const endDate = document.getElementById("end_date").value;
      const depth = 2;
      const category = document
        .getElementById("category")
        .value.replaceAll(" ", "_");
      const article = document
        .getElementById("article")
        .value.replaceAll(" ", "_");

      let apiUrl = `/api/pageviews/${type}?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}&depth=${depth}`;
      let label = "";

      if (type == "article") {
        apiUrl += `&article=${encodeURIComponent(article)}`;
        label = `Article: ${wiki} - ${article}`;
      }
      if (type == "category") {
        apiUrl += `&category=${encodeURIComponent(category)}`;
        label = `Category: ${wiki} - ${category}`;
      }

      try {
        const startTime = performance.now();
        removeMessage();

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
        showMessage("Failed to fetch data. Please try again.", "error");
      }
    });
});

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

  setupAutocomplete("category", "/api/search/categories");
  setupAutocomplete("article", "/api/search/articles");
});

async function setupAutocomplete(inputId, apiUrl) {
  const inputField = document.getElementById(inputId);
  inputField.addEventListener("input", async () => {
    const dataList = document.getElementById(`datalist-${inputId}`);
    const query = inputField.value;
    if (query.length < 2) return; // Only search for 2+ characters

    try {
      const response = await fetch(
        `${apiUrl}?${inputId}=${encodeURIComponent(query)}&wiki=${wiki.value}`,
      );
      if (!response.ok) {
        console.error("Failed to fetch autocomplete data");
        return;
      }

      const suggestions = await response.json();
      dataList.innerHTML = ""; // Clear previous suggestions
      suggestions.forEach((item) => {
        const option = document.createElement("option");
        option.value = item.replaceAll("_", " ");
        dataList.appendChild(option);
      });
    } catch (error) {
      console.error("Error fetching autocomplete data:", error);
    }
  });
}

function removeMessage() {
  const messageContainer = document.getElementById("message-container");
  if (messageContainer) {
    messageContainer.remove();
  }
}

function showMessage(message, type) {
  const sidebar = document.querySelector(".sidebar");
  const messageDiv = document.createElement("div");
  messageDiv.id = "message-container";
  messageDiv.classList.add(
    type === "error" ? "error-message" : "success-message",
  );
  messageDiv.textContent = message;
  sidebar.appendChild(messageDiv);
}
