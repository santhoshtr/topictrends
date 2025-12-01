SELECT
    p.page_id,
    CAST(SUBSTRING(pp_value, 2) AS UNSIGNED) AS qid,
    p.page_title
FROM
    PAGE p
    JOIN page_props ON pp_page = page_id
    AND pp_propname = 'wikibase_item'
WHERE
    p.page_namespace = 14 -- 14 is the namespace for categories
    AND p.page_id NOT IN (
        SELECT
            pp_page
        FROM
            page_props
        WHERE
            pp_propname = 'hiddencat'
    )
