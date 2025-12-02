SELECT
    p.page_title
FROM
    page p
    JOIN page_props ON pp_page = p.page_id
    AND pp_propname = 'wikibase_item'
WHERE
    pp_value = ?
LIMIT
    1
