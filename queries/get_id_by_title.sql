SELECT
    page_id
FROM
    PAGE
WHERE
    page_title = ?
    AND page_namespace = ?
LIMIT
    1
