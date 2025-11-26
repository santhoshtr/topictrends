document.addEventListener("DOMContentLoaded", function () {
  initializeChart();

  document
    .getElementById("trend-form")
    .addEventListener("submit", async (event) => {
      event.preventDefault();

      const type = document.getElementById("type").value;
      const wiki = document.getElementById("wiki").value;
      const startDate = document.getElementById("start_date").value;
      const endDate = document.getElementById("end_date").value;
      const category = document.getElementById("category").value;
      const article = document.getElementById("article").value;

      let apiUrl = `/api/pageviews/${type}?wiki=${wiki}&start_date=${startDate}&end_date=${endDate}`;
      let label = "";

      if (type == "article") {
        apiUrl += `&article=${encodeURIComponent(article)}`;
        label = `Article: ${article}`;
      }
      if (type == "category") {
        apiUrl += `&category=${encodeURIComponent(category)}`;
        label = `Category: ${category}`;
      }

      try {
        const response = await fetch(apiUrl);
        if (!response.ok) {
          throw new Error("Failed to fetch data");
        }

        const data = await response.json();
        updateChart(data, label);
      } catch (error) {
        console.error("Error:", error);
        alert("Failed to fetch data. Please try again.");
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
});
