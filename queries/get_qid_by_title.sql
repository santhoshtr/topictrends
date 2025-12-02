SELECT
    CAST(SUBSTRING(pp_value, 2) AS UNSIGNED) AS qid
FROM
    page p
    JOIN page_props ON pp_page = page_id
    AND pp_propname = 'wikibase_item'
WHERE
    page_title = ?
    AND page_namespace = ?
LIMIT
    1
