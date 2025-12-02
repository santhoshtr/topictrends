SELECT
    CAST(SUBSTRING(pp_value, 2) AS UNSIGNED) AS qid,
    p.page_title
FROM
    page p
    JOIN page_props ON pp_page = p.page_id
    AND pp_propname = 'wikibase_item'
WHERE
    pp_value IN ({})
