SELECT
    p.page_id,
    CAST(SUBSTRING(pp_value, 2) AS UNSIGNED) AS qid,
    p.page_title
FROM
    page p
    JOIN page_props ON pp_page = page_id
    AND pp_propname = 'wikibase_item'
WHERE
    p.page_namespace = 0 
    AND p.page_is_redirect = 0
    AND p.page_is_new = 0

