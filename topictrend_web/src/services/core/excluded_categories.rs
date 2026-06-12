//! Curated category-QID denylist applied at ranking surfaces.
//!
//! Maintenance, content-organization, and whole-population classification
//! categories saturate the top of any category ranking (gap discovery,
//! trending) without being actionable or informative. They are excluded from
//! *rankings only* — direct lookups (a category's trend, its article list)
//! are unaffected, and the underlying data keeps the edges.
//!
//! This is editorial judgment over visible categories, complementing the
//! hiddencat exclusion done at ETL fetch time (which only removes categories
//! the wikis themselves flag as hidden). High cross-wiki agreement does not
//! help here: "Living people" is asserted by many wikis and still noise in a
//! ranking.
//!
//! Curated from the enwiki→hiwiki gap response (the largest reference);
//! unlabeled entries were identified via Wikidata. Add new ones as they
//! surface.

use std::collections::HashSet;
use std::sync::LazyLock;

pub static EXCLUDED_CATEGORY_QIDS: LazyLock<HashSet<u32>> = LazyLock::new(|| {
    [
        // Whole-population / biographical classification
        5312304, // Living people
        4047087, // People
        9507857, // Men
        7473085, // Women
        6697530, // Humans
        7045213, // Surnames
        // Disambiguation
        1982926, // Disambiguation pages
        9700479, // All disambiguation pages
        4671251, // Human name disambiguation pages
        4671284, // Place name disambiguation pages
        8379354, // Disambiguation pages with surname-holder lists
        // Stubs
        2944440, // Stubs
        7046360, // Biology stubs
        7046440, // Geography stubs
        5834688, // People stubs / incomplete biographies
        130866438, // Stub articles (ug)
        // Wikipedia maintenance / templates
        130251703, // Pages with image sizes containing extra px
        3740,      // Wikipedia templates
        6332021,   // Articles in translation
        18285010,  // Bot created articles from 2013-02
        22165254,  // Robot created butterfly items
        // Tracking categories that surfaced in canonical-topology trending —
        // visible (not hiddencat-flagged) on their home wikis.
        27892622, // Webarchive template wayback links
        10152088, // Pages with reference errors
        7478359,  // Articles lacking sources
        10051136, // Articles to be expanded
        4989282,  // Pages with broken file links
        9806171,  // Öömrang articles (frrwiki language-variant tracking)
        8922197,  // Wikipedia articles with LCCN identifiers
        8922195,  // Wikipedia articles with GND identifiers
        27825420, // Pages using ISBN magic links
        8181072,  // List of articles every Wikipedia should have
        4387444,  // Featured articles (project namespace)
        6157677,  // Spoken Wikipedia
        // "By alphabetical order" / by-name organizational containers
        32889963, // People by alphabetical order
        6547581,  // Populated places by alphabet
        9961681,  // Sportspeople by name
        54860644, // Administrative subdivisions in alphabetical order
        9700775,  // Footballers by alphabetical order
    ]
    .into_iter()
    .collect()
});

/// How many results to request from an engine so that filtering out excluded
/// categories cannot shrink a page below `limit` (at most the whole denylist
/// can be dropped).
pub fn oversampled(limit: usize) -> usize {
    limit + EXCLUDED_CATEGORY_QIDS.len()
}
