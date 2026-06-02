-- Edit counts per main-namespace article for a single day, from the replica.
-- @DAY / @NEXTDAY are YYYYMMDD bounds substituted by the Makefile; the
-- half-open [@DAY, @NEXTDAY) range over the indexed rev_timestamp yields
-- exactly one complete day. Aggregated server-side so only one row per page
-- crosses the wire. page_id → qid translation happens in get-day-pageedits.
SELECT
    rev_page AS page_id,
    COUNT(*) AS edit_count
FROM
    revision
    JOIN page ON page_id = rev_page
WHERE
    page_namespace = 0
    AND page_is_redirect = 0
    AND rev_timestamp >= '@DAY000000'
    AND rev_timestamp < '@NEXTDAY000000'
GROUP BY
    rev_page
