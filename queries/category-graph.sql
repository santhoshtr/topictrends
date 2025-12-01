SELECT
    cl.cl_from AS category,
    p.page_id AS category_page_id
FROM
    categorylinks cl
    JOIN linktarget lt ON lt.lt_id = cl.cl_target_id
    JOIN PAGE p ON lt.lt_title = p.page_title
    AND p.page_namespace = 14
WHERE
    lt.lt_namespace = 14
    AND cl.cl_type = 'subcat'
