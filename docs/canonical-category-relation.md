# The cross-wiki canonical category relation

A technical report on what 337 Wikipedia editions collectively know about
categorizing their articles.

Snapshot: 2026-06-11 (337 editions, `data/wikipedia.list`, cebwiki excluded).
Author: Santhosh Thottingal

---

## 1. Summary

Every Wikipedia edition maintains its own category system, and they differ
wildly in depth, density, and philosophy: enwiki builds deep, narrow
intersection trees ("1950 in computing"), while small editions file articles
directly under broad concepts ("Sports"). Because both articles and categories
are linked across editions by Wikidata QIDs, the 337 per-edition
article→category relations can be unioned into a single **canonical relation**.

The headline numbers, measured on the 2026-06-11 snapshot:

- **337 editions**, **238.5M raw** (article, category) assignments → **146.76M
  distinct canonical edges**, each carrying a `wiki_count` (how many editions
  independently make that assignment).
- The agreement distribution is extremely skewed: **76.05% of canonical edges
  are single-wiki claims**, 88.85% are made by at most two editions.
  Categorization effort across editions is largely *complementary*, not
  duplicated.
- This skew is the central design fact. A naive "trust only what ≥2 wikis agree
  on" default would delete three quarters of the relation — and most of the
  small-wiki recall the relation exists to provide.
- The union recovers categorizations that an edition's *own* category graph
  cannot reach. Example: "Machine learning" does not exist as a category in
  mlwiki or hiwiki at any traversal depth (v1 = 0), yet the canonical relation
  yields **31 / 59** correct member articles for those editions respectively.
- The union also relocates the classic *saturation* problem rather than solving
  it for free. enwiki's "Sports" category has **132,552** direct canonical
  members at agreement `k=1`; requiring `k≥2` collapses this to **1,150** clean,
  correctly-ranked members. Agreement count `k` is the graded knob that
  hierarchy depth was the blunt instrument for.

The rest of this report characterizes the relation (§3), shows the agreement
count as a relevance signal through worked examples (§4), develops the
flattening / saturation-migration phenomenon (§5), decomposes coverage gaps into
*structure* vs *content* (§6), and lists the measurement biases (§7).

---

## 2. The instrument: how the canonical relation is built

TopicTrends models each edition's category graph in memory as Compressed Sparse
Row matrices over `u32` Wikidata QIDs (see `ARCHITECTURE.md` for the engine).
The canonical relation is one further step on top of that:

1. For each edition `W`, take its `article_category` relation — the depth-0
   direct assignments of articles to categories, both keyed by QID.
2. Union across all 337 editions, keying each edge by the `(article_qid,
   category_qid)` pair.
3. Count multiplicity: `wiki_count(a, c)` = how many editions independently file
   article `a` under category `c`.

The result is a single relation of 146.76M edges. Two derived sets matter
throughout:

- `canonical_set(C)` — the union of direct members of category `C` across all
  editions.
- `canonical ∩ W` — the members of `canonical_set(C)` that exist as articles in
  edition `W` (optionally filtered to `wiki_count ≥ k`).

### Build cost

The union is cheap. Reading all 337 `article_category` parquets, packing each
`(article, category)` pair into a `u64`, sorting, and run-length-counting
multiplicity processed **238M raw rows → 146.8M distinct edges in minutes** on a
14-core / 32GB machine, well within memory. The relation is not a heavyweight
precompute; it can be rebuilt on every topology refresh.

### Hidden / maintenance-category filtering

Stub, tracking, and bot-maintenance categories ("People stubs", "Robotskapade
artiklar 2016-06", "Pages with script errors") carry Wikidata QIDs and therefore
enter the union as ordinary edges. MediaWiki marks these `__HIDDENCAT__`
(`page_props.hiddencat`); the per-edition fetch query excludes hidden categories
at source.

The filter's effect is **per-wiki asymmetric**, and the asymmetry is itself a
finding:

- On **enwiki** it is nearly a no-op. Of 91,882 hidden categories, 35,798 carry
  a QID — but their members are templates and project pages already excluded by
  the namespace-0 join. Total article edges removed: **−584**.
- On **bot-farm editions** it does real work. svwiki's hidden "Robotskapade"
  (bot-created) tracking categories carry tens of thousands of *article* edges
  each (e.g. Q25692949: 30,010; Q25692962: 30,951; many monthly variants), all
  dropped at fetch.

Visible meta-categories ("Living people" — confirmed *not* hidden on enwiki —
"People stubs") pass through by design. For those, the agreement threshold `k`
and a curated denylist in the coverage UI remain the tools, not the hidden-cat
filter.

---

## 3. The shape of the relation

### 3.1 Agreement distribution

The `wiki_count` histogram over all 146.76M canonical edges:

| wiki_count | edges | % of edges | cumulative |
|---:|---:|---:|---:|
| 1 | 111.60M | 76.05% | 76.05% |
| 2 | 18.80M | 12.81% | 88.85% |
| 3 | 6.78M | 4.62% | 93.47% |
| 4 | 3.26M | 2.22% | 95.69% |
| 5+ | 6.33M | 4.31% | 100% |

**76% of all canonical edges are single-wiki claims.** This is the defining
property of the relation. It means:

- The union is overwhelmingly *additive* — most editions contribute
  categorizations no other edition has. The 337 systems are complementary, not
  redundant copies.
- Agreement is rare and therefore informative. When many editions independently
  file an article under the same category, that coincidence is a strong signal
  (§4); when only one does, it is usually correct but unranked (the count=1
  tail).
- Any consumer that filters by agreement must default to `k=1` and treat `k` as
  a precision knob, not a gate (§5).

<!-- FIGURE: docs/figures/wiki_count_hist.svg — log-scale bar of the above. GEN. -->

### 3.2 Per-edition morphology

<!-- GEN pass A — not yet run. Fill from a per-edition aggregation over the 337
topology parquets (reuse the v2-study histogram machinery and
analyze_depth_from_root). Numbers below are PLACEHOLDERS to be replaced. -->

The qualitative claim, to be backed with measured per-edition numbers: category
systems form a continuum from **deep-intersection** (enwiki: many categories per
article, deep hierarchies, narrow intersection categories) to **flat-broad**
(small editions: few categories per article, shallow trees, articles filed
directly under broad concepts).

Metrics to report per edition, for ~15–20 editions spanning the size range
(enwiki, dewiki, … down to mlwiki, hiwiki, and the bot-farm svwiki):

- categories per article (mean, median);
- direct-members-per-category distribution;
- hierarchy depth / branching from the category root;
- fraction of "flat" assignments (articles whose only categories are top-level).

> **TODO (GEN pass A):** populate this table. Code: extend `.plans/v2_study.rs`
> with a `morphology` mode, or a small new aggregation; data is local (337
> `article_category.parquet` + `category_graph.parquet`).

### 3.3 Non-redundancy and the union gain

<!-- GEN pass A — same pass. -->

The 76%-single-wiki figure (3.1) restated per edition: how much does the union
*expand* each edition's local relation? For a small edition this gain is large
(it inherits categorizations from the entire corpus); for enwiki it is small
(enwiki already supplies much of the union). The report should give:

- per-edition `|canonical ∩ W| / |local direct members|` gain factor vs edition
  size;
- which editions are net *exporters* (their single-wiki claims dominate the
  union) vs *importers* (they gain most from it).

> **TODO (GEN pass A):** populate. Net-exporter ranking falls out of attributing
> each `k=1` edge to its sole contributing wiki.

### 3.4 QID-linkage coverage (a bias, reported honestly)

<!-- GEN pass A — same pass. -->

Categories and articles without a Wikidata item are invisible to the union: they
cannot be matched across editions and simply drop out. This biases the relation
toward concepts that have made it into Wikidata. The report must state, per
edition, the fraction of categories (and of category assignments) that lack a
QID and are therefore excluded.

> **TODO (GEN pass A):** count category rows lacking a QID per edition. Report
> the number even where it is unflattering — it bounds the relation's coverage.

---

## 4. Agreement count as a relevance signal

The `wiki_count` on each edge is an intrinsic, language-agnostic signal: no
model, no training data, just the count of editions that independently made the
same assignment. This section shows what ranking by that count surfaces, through
the article → ranked-categories direction. These are illustrative worked
examples, not a precision/recall evaluation (we have no labeled set; see §7).

- **Alan Turing** (Q7251, categorized by 147 editions) — top ranked categories:
  1912 births (108), 1954 deaths (106), Fellows of the Royal Society (41),
  English mathematicians (37), Category:Alan Turing (32), OBE (31), Princeton
  alumni (31)… Topical signal is strong immediately after the birth/death
  boilerplate. Of 343 distinct categories, the `count=1` tail is mostly
  per-edition year/calendar variants — noise, but harmless under ranking.
- **Jeffrey Epstein** (Q2904131, 76 editions) — 1953 births (60), 2019 deaths
  (60), then **American criminals (26)** > American businesspeople (25) >
  American Jews (25). Consensus ranks the criminal categorization above any
  single-edition outlier claim(enwiki's Physics Educator) — the motivating example for the signal.
- **Salim Kumar** (Q7404571, 11 editions) — 1969 births (11), 2026 deaths (9),
  **Male actors in Malayalam cinema (7)**, Best Actor National Film Award
  winners (6). The tail includes stale "Living people" (3 editions) and "Recent
  deaths"; consensus correctly outvotes the stale claims. This is the
  *contested-categorization* case: disagreement between editions is visible in
  the count, and the majority is right here.
- **Manchester city centre** (Q2166304, 9 sitelinks) — Manchester (5), Central
  business districts in the UK (3), Areas of Manchester (2). Clean even at low
  counts.
- **Humphrey Chetham** (Q5941334, 3 sitelinks) — 1653 deaths / 1580 births (3),
  then a `count=1` tail that is still *signal*, not noise (High sheriffs of
  Lancashire, 17th-century English merchants), sourced almost entirely from
  enwiki. Degrades gracefully.
- **Williams Middle School** (Q8021066, enwiki-only) — all 8 categories at
  `count=1`; ranking degenerates to ties exactly as expected for a
  single-edition article. The content is still correct and usable, but consumers
  must see the count to read the tie structure. (This is why the API carries the
  count, not just an ordered list.)

What these show: the count separates topical categorization from boilerplate and
from single-edition outliers when enough editions participate, and it degrades to
honest ties when they don't. What they do *not* show, and we do not claim, is a
quantified precision across agreement bands — that needs multilingual human
judgment we did not perform. The correlation between `count=1` outlier claims and
edit-war / revert signals (we retain edit histories but have not joined them) is
likewise left as future work.

---

## 5. Crowd-sourced flattening and saturation migration

This is the central phenomenon. The worked examples below are **n=10 categories
× 4 editions** (enwiki, mlwiki, tawiki, hiwiki), chosen to span the regimes
(broad / saturated / mid-size / narrow / eponymous / current-events /
local-interest). They illustrate the phenomenon; they are not a large-n sweep
(that is deferred — see §8).

Terms: `canonical ∩ W (k=n)` = members of `W` in the union with ≥ n editions
agreeing; `d0/d1/d2` = the v1 engine's `get_articles_in_category` at traversal
depth 0/1/2; `recovered` = `canonical ∩ W − d2`; `lost` = `d2 − canonical ∩ W`.

### 5.1 The saturation migration

In the v1 (single-edition) engine, broad abstract categories *saturate* under
depth traversal: from "Philosophy" or "Sports", depth-2 traversal absorbs nearly
everything. The canonical relation does **not** make this go away — it relocates
it from *depth* to *direct-assignment breadth*, because shallow editions file
every athlete and club directly under "Sports". The antidote changes from
traversal depth to the agreement count `k`:

| category | canonical (all) | en k=1 | en k≥2 | en d2 | ml k=1 | ml d2 | ta k=1 | ta d2 | hi k=1 | hi d2 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Sports (Q1457982) | 142,795 | 132,552 | 1,150 | 10,000\* | 1,053 | 144 | 1,339 | 1,764 | 1,698 | 3,889 |
| Physics (Q1457258) | 8,707 | 5,109 | 1,697 | 7,509 | 1,271 | 1,350 | 1,741 | 2,359 | 1,980 | 2,032 |
| Literature (Q8259) | 9,385 | 5,742 | 917 | 15,816 | 795 | 2,285 | 723 | 2,444 | 1,175 | 3,941 |

\* enwiki d2 output capped at 10,000; the true value is larger.

Sports' canonical set is 142K because shallow editions file everything directly
under it; enwiki alone gives 132,552 at `k=1`. But `k≥2` collapses this to
**1,150**, and the resulting ranking is clean: Sport (179), Association football
(68), Basketball (49), Olympic Games (47) — and "Olympic Games" is *not*
reachable from enwiki's own Sports subtree at depth 2. The `k` knob does for the
union what depth did for the single edition, except graded by evidence instead of
hop count. Meanwhile v1's depth-2 surplus is mostly junk (Oxygen, Genghis Khan,
Backpack under Sports; Star, Planet under Physics; Internet Archive under
Literature).

**The one shape that needs the escape hatch:** raw `k=1` membership of a giant
abstract category(Example: Science, Philosophy, Maths) is unusable. Every *ranked* / top-N use is fine (it gets the
`k`-grading for free); only "list every direct member of Sports" needs `k≥2` or
the local hierarchy.

### 5.2 Where the union clearly wins

**Artificial intelligence** (Q558331, 91 editions instantiate it):

| wiki | k=1 | k≥2 | d0 | d1 | d2 | Jaccard | recovered | lost |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| enwiki | 1684 | 762 | 238 | 1488 | 4756 | 0.192 | 647 | 3719 |
| mlwiki | 91 | 63 | 20 | 31 | 33 | 0.228 | 68 | 10 |
| tawiki | 135 | 85 | 21 | 30 | 31 | 0.230 | 104 | 0 |
| hiwiki | 140 | 95 | 21 | 45 | 105 | 0.150 | 108 | 73 |

Small editions get 3–4× more articles, with pristine top-20 sets (OpenAI,
ChatGPT, neural network, computer vision, machine translation — none reachable by
mlwiki's own graph). On enwiki the canonical set is *smaller* than d2, but d2's
surplus is traversal junk: its "lost" set includes Big Bang, Star Wars, JPEG,
and Wikidata, while the canonical relation *recovers* Deep learning, Neural
network, Genetic algorithm, and DALL-E that enwiki's own subtree misses at depth
2. Lower recall than d2 here is precision, not loss.

**Machine learning** (Q7015116) — the headline small-wiki result:

| wiki | k=1 | k≥2 | d0 | d1 | d2 | recovered | lost |
|---|---:|---:|---:|---:|---:|---:|---:|
| enwiki | 870 | 476 | 269 | 1420 | 1877 | 217 | 1224 |
| mlwiki | 31 | 24 | **0** | **0** | **0** | 31 | 0 |
| tawiki | 39 | 28 | 2 | 2 | 2 | 37 | 0 |
| hiwiki | 59 | 36 | **0** | **0** | **0** | 59 | 0 |

The category **does not exist in mlwiki or hiwiki** (v1 = 0 at every depth), yet
the canonical relation yields 31 / 59 correct articles — including യന്ത്രപഠനം
(Machine learning itself), Deep learning, pattern recognition, TensorFlow,
PyTorch. This is the premise of the whole approach demonstrated end to end.

**2026 deaths** (Q9725487) — freshness propagation, the strongest
trending-use-case evidence:

| wiki | k=1 | d2 | recovered | lost |
|---|---:|---:|---:|---:|
| enwiki | 4744 | 4676 | 104 | 36 |
| mlwiki | 64 | 8 | 56 | 0 |
| tawiki | 91 | 58 | 33 | 0 |
| hiwiki | 60 | 15 | 45 | 0 |

mlwiki goes 8 → 64 (Khamenei, Habermas, Jesse Jackson, Asha Bhosle — all dead in
2026 by consensus, none yet categorized in mlwiki). enwiki's Jaccard is 0.971:
for a flat, well-maintained category, canonical ≈ local, as expected. enwiki's 36
"lost" are its "2026 racehorse deaths" subcategory (Maybe, Miss Finland — horses).

**Alan Turing** (Q9384007, eponymous) — enwiki canonical 82 vs d2 281; d2's
surplus is the Turing *Award* laureates subtree (Knuth, Dijkstra, Berners-Lee —
not about Alan Turing), while canonical recovers Halting problem, Enigma machine,
Bletchley Park. Small editions: v1 = 0 everywhere; canonical gives 7 (ml) / 4
(ta) / 7 (hi). Eponymous categories are exactly where depth traversal misleads.

### 5.3 The honest counter-cases

The union is not universally better. Three regimes where a curated local subtree
or the local hierarchy wins, kept prominent rather than buried:

**Nobel laureates** (Q6635159) — the enwiki completeness counter-result:

| wiki | k=1 | k≥2 | d0 | d1 | d2 | recovered | lost |
|---|---:|---:|---:|---:|---:|---:|---:|
| enwiki | 1105 | 710 | 9 | 1051 | 1688 | 122 | 705 |
| mlwiki | 515 | 349 | 5 | 50 | 388 | 152 | 25 |
| tawiki | 625 | 410 | 2 | 553 | 567 | 120 | 62 |
| hiwiki | 704 | 474 | 178 | 474 | 479 | 261 | 36 |

Real laureates live only in field subcategories ("Nobel laureates in
Chemistry"), so the union loses ~705 d2 members including genuine laureates
(Carolyn Bertozzi). The union recovers what *some* edition files directly (Curie,
Mandela, Einstein) but not enwiki's full subtree rollup. Union noise is also
visible: "The Old Man and the Sea" appears (6 editions file the novel under Nobel
laureates). Small editions still net-win (hi 479 → 704, ml 388 → 515). For
completeness-critical queries on enwiki-style trees, the kept local hierarchy
remains the right tool.

**Male actors in Malayalam cinema** (Q15271862) — the strongest counter-case:

| wiki | k=1 | k≥2 | d0 | d1 | d2 | Jaccard | recovered | lost |
|---|---:|---:|---:|---:|---:|---:|---:|
| enwiki | 733 | 605 | 681 | 681 | 681 | 0.929 | 52 | 0 |
| mlwiki | 516 | 421 | 385 | 387 | 387 | 0.747 | 130 | 1 |
| tawiki | 307 | 265 | 182 | **768** | **773** | 0.207 | 122 | **588** |
| hiwiki | 112 | 95 | **0** | **0** | **0** | 0.000 | 112 | 0 |

mlwiki gains 130 actors its own community never categorized locally (Jackie
Shroff, R. Madhavan, Amrish Puri — cross-industry actors other editions file
here); hiwiki goes 0 → 112. But **tawiki's local subtree yields 773 vs the
union's 307** — 588 "lost". tawiki has invested in deep sub-categorization here
(its d1 alone is 768). If those 588 are genuinely Malayalam-cinema actors, this
repeats the Nobel pattern: a locally-curated subtree beats the union.

> **Open item (must resolve before this report is final):** spot-check whether
> tawiki's 588 lost members are genuine Malayalam-cinema actors or subtree
> over-reach. Reproduce with `v2-study compare --wiki tawiki --categories
> 15271862`. The verdict decides how loudly we advertise local-hierarchy
> rollup.

**1950 in computing** (Q25304526, narrow enwiki intersection) — the expected
regression: canonical = **3** (Turing test ×5 editions, Hamming distance,
Computing Machinery and Intelligence) vs enwiki d2 = 15 (UNIVAC 1101, Pilot ACE,
SEAC…). Long-tail intersection categories stay one-edition-local and lose their
subtree. The 3 direct members are semantically correct; ml/ta/hi give 1/0/1.
Acceptable — these categories are navigation aids, not analytics targets, and the
local hierarchy still exists for them.

### 5.4 Reading of §5

The phenomenon: depth-traversal saturation and the precision loss of broad
categories *migrate* into the union as direct-assignment breadth, and the
agreement count `k` is the graded antidote that hierarchy depth was the blunt
version of. The union wins decisively on small editions (frequently from a
literal zero) and on freshness; it loses on completeness-critical queries over
rich, well-curated enwiki subtrees, which is precisely where the local hierarchy
should be kept. The design consequence (k=1 default, k as precision knob, keep
local hierarchy as an escape hatch) is in §8.

---

## 6. Structure gap vs content gap

A coverage gap for a topic in an edition has two distinct causes that a single
"gap" number conflates:

- **Structure gap** — the articles may exist in the edition, but are not wired
  into the category system (category missing, or not populated).
- **Content gap** — the articles do not exist in the edition at all.

These need different editor responses (recategorization vs translation/creation),
so the report decomposes them with two depth-0 measures, materialized as the
coverage matrix (`data/{wiki}/coverage/{snapshot}.parquet`):

1. **`direct_coverage`** — depth-0 direct members of category `C` in edition `W`.
   A divergence here is a *structure / categorization* gap. Unambiguous,
   non-saturating, refresh-stable.
2. **`qid_overlap_coverage`** — over the canonical set
   `canonical_set(C) = ⋃_wikis directmembers(C)`, count how many of those
   articles exist in `W` at all, *ignoring how W categorizes them*. This is the
   pure *content* gap, independent of W's structure.

The matrix's own **row-max is a free denominator** — no privileged reference
edition. Recursive (subtree) coverage is deliberately omitted because the correct
traversal depth is per-category (depth-0 understates broad topics; no-cap
saturates them), so no fixed depth is right for a published snapshot; analysts
roll up a chosen subtree from the edge list on demand instead.

### 6.1 Per-edition equity profile

<!-- GEN pass B — not yet run. Fill from the materialized coverage matrix
(data/*/coverage/2026-06-15.parquet, present locally). The "Science 4%, Cinema
31%" figures below are ILLUSTRATIVE and must be replaced with measured values. -->

Rolling the matrix up to top-level categories gives a knowledge-equity profile
per edition: "edition W covers X% of globally-known Science topics, Y% of
Cinema," comparable across editions.

> **TODO (GEN pass B):** compute real per-edition rollups (replace the
> illustrative "tawiki: Science 4%, Cinema 31%"). Data is local.

### 6.2 Interest fingerprints

<!-- GEN pass B — same matrix. -->

Cells where an edition *exceeds* the row-max of all others (over-coverage)
surface that community's local-interest topics — a quantitative map of cultural
specificity per language community.

> **TODO (GEN pass B):** list, per edition, the categories where it exceeds the
> row-max elsewhere.

---

## 7. Measurement biases and limitations

Stated plainly, as first-class content:

- **QID-linkage coverage.** Categories and articles without a Wikidata item are
  invisible to the union and silently dropped. This biases the relation toward
  Wikidata-linked concepts. (Per-edition magnitude: §3.4, GEN pass A.)
- **cebwiki excluded.** The Cebuano edition, heavily bot-generated, is excluded
  from this snapshot's universe. Its inclusion would inflate single-wiki edge
  counts.
- **Hidden-category policy is asymmetric.** Hidden/maintenance categories are
  filtered at fetch, but the effect ranges from −584 edges on enwiki to ~30K per
  category on svwiki (§2). Visible meta-categories ("Living people") are *not*
  filtered and pass into the union; `k` and a curated denylist handle those.
- **No longitudinal snapshots.** We retain individual dated snapshots but have no
  long history, so this report makes no claims about *dynamics* — propagation
  latency, gap-closing over time. Those require retention started now (§8).
- **Worked-example scale.** §5's category-level recall/precision reading rests on
  10 categories × 4 editions, chosen to span the regimes. It is an illustration
  of the phenomenon, not a large-n measurement.
- **No human evaluation.** §4's "relevance" and §5's "junk" / "pristine"
  judgments are the author's reading of the member lists, not multilingual rater
  judgments. No precision/recall numbers are claimed where ground truth would be
  needed.
- **English-first labels.** Category and article labels in this report are shown
  English-first for readability; this is presentation only and does not affect
  the QID-keyed computation.

---

## 8. What this enables, and future work

### 8.1 Engine design decisions this study de-risked

- **`k=1` default, `k` as a precision knob, not a gate.** 76% of edges are
  single-wiki; defaulting to `k≥2` would destroy the relation's value. But for
  saturated abstract categories (Sports 132K → 1,150) the knob is the difference
  between unusable and excellent (§5.1). Engines that rank by `wiki_count` get
  the grading for free.
- **Keep the local hierarchy as an escape hatch.** For completeness-critical
  queries over rich enwiki subtrees (Nobel laureates, possibly tawiki's actor
  subtree), local depth-traversal rollup remains the right tool (§5.3).
- **Ranked output must carry the count.** Ties at `count=1` are common for
  low-sitelink articles; consumers need the count to read the tie structure
  (§4, Williams Middle School).
- **Filter hidden categories at ETL** (§2), but do not expect it to handle
  visible meta-categories.

### 8.2 Future work

- **Longitudinal dynamics.** With monthly canonical-snapshot retention (the v2
  ETL already keys outputs by date; retention is the decision), the relation
  supports propagation-latency analysis — when an event recategorizes an article
  ("death" → "2026 deaths"), how fast do editions follow — and gap-closing /
  emerging-category tracking via snapshot diffs. Pageview spikes supply event
  timestamps for free. Not attempted here for lack of history.
- **Large-n flattening sweep.** §5's phenomenon at scale: a category sampler over
  `v2-study compare`, stratified by enwiki direct size and number of
  instantiating editions, aggregating recovered/lost/Jaccard distributions over
  thousands of categories. Deferred from this first report copy.
- **Consensus vs edit-history.** Join `count=1` outlier claims against edit-war /
  revert signals to test whether agreement detects contested categorization
  (§4). Edit histories are retained but not yet joined.
- **Reusable artifact.** The 146.8M-edge relation with agreement counts is usable
  on its own. Dataset-release groundwork is sketched separately; a release should
  publish a frozen, dump-derived snapshot with a documented date so the numbers
  here are regenerable without replica access.

---

## Appendix: reproducing the numbers

Snapshot: 2026-06-11, 337 editions (`data/wikipedia.list`, cebwiki excluded),
full topology parquets per edition.

```bash
# wiki_count histogram (§3.1)
target/release/v2-study histogram

# per-category comparison (§5 tables)
target/release/v2-study compare --wiki mlwiki  --categories 7015116
target/release/v2-study compare --wiki enwiki  --categories 1457982,1457258,8259
target/release/v2-study compare --wiki tawiki  --categories 15271862

# article → ranked categories (§4)
target/release/article-categories --qid 7251     # Alan Turing
target/release/article-categories --qid 2904131  # Jeffrey Epstein
```

The study harness is `topictrend_cli/src/v2_study.rs` (a throwaway `[[bin]]`, modes
`compare` and `histogram`). Coverage-matrix snapshots are under
`data/{wiki}/coverage/{snapshot}.parquet`. A reader without Wikimedia replica
access reproduces from the published relation artifact, not the ETL.
