SELECT
    cl.cl_from AS article_qid,
    p.page_id AS category_page_id
FROM
    categorylinks cl
    JOIN linktarget lt ON lt.lt_id = cl.cl_target_id
    JOIN page p ON lt.lt_title = p.page_title
    AND lt.lt_namespace = 14
    AND p.page_namespace = 14
    -- Exclude hidden (maintenance/tracking) categories: stubs, bot-created
    -- markers, "Living people" etc. carry QIDs and pollute the canonical
    -- cross-wiki union. PK lookup on page_props, cheap per row.
    LEFT JOIN page_props pp ON pp.pp_page = p.page_id
    AND pp.pp_propname = 'hiddencat'
WHERE
    lt.lt_namespace = 14
    AND cl.cl_type = 'page'
    AND pp.pp_page IS NULL
