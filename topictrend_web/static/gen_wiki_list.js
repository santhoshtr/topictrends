const fs = require("fs");
const https = require("https");

async function fetchWikiList() {
  return new Promise((resolve, reject) => {
    const url =
      "https://meta.wikimedia.org/w/api.php?action=sitematrix&format=json";

    const options = {
      headers: {
        "User-Agent":
          "TopicTrend/1.0 (https://topictrends.wmcloud.org; contact@example.com)",
      },
    };

    https
      .get(url, options, (res) => {
        let data = "";

        res.on("data", (chunk) => {
          data += chunk;
        });

        res.on("end", () => {
          try {
            const response = JSON.parse(data);
            resolve(response);
          } catch (error) {
            reject(error);
          }
        });
      })
      .on("error", (error) => {
        reject(error);
      });
  });
}

async function generateWikiList() {
  try {
    console.log("Fetching wiki list from Wikimedia...");
    const data = await fetchWikiList();

    const wikis = [];

    // Parse the sitematrix response
    Object.keys(data.sitematrix).forEach((key) => {
      if (key !== "count" && key !== "specials") {
        const languageGroup = data.sitematrix[key];

        // Check if languageGroup has the expected structure
        if (
          languageGroup &&
          languageGroup.site &&
          Array.isArray(languageGroup.site)
        ) {
          languageGroup.site.forEach((site) => {
            // Check if it's a Wikipedia site and not closed
            if (
              site.code === "wiki" &&
              !site.hasOwnProperty("closed") && // closed property doesn't exist for active sites
              !site.hasOwnProperty("private")
            ) {
              // private property doesn't exist for public sites

              wikis.push({
                code: `${languageGroup.code}wiki`,
                name: languageGroup.name,
                localname: languageGroup.localname || languageGroup.name,
                langcode: languageGroup.code,
                url: site.url,
                dbname: site.dbname,
              });
            }
          });
        }
      }
    });

    // Remove duplicates based on code
    const uniqueWikis = wikis.filter(
      (wiki, index, self) =>
        index === self.findIndex((w) => w.code === wiki.code),
    );

    // Sort by English name
    uniqueWikis.sort((a, b) => a.name.localeCompare(b.name));

    // Write to static JSON file
    const outputPath = "./topictrend_web/static/wikis.json";
    fs.writeFileSync(outputPath, JSON.stringify(uniqueWikis, null, 2));

    console.log(`Generated wiki list with ${uniqueWikis.length} wikis`);
    console.log(`Saved to: ${outputPath}`);
  } catch (error) {
    console.error("Error generating wiki list:", error);
    console.error("Error details:", error.message);
    process.exit(1);
  }
}
generateWikiList();
