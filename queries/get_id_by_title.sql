SELECT
    page_id
FROM
    page
WHERE
    page_title = ?
    AND page_namespace = ?
LIMIT
    1
