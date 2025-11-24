SELECT cl.cl_from AS article_id, p.page_id as category_id 
FROM categorylinks cl 
JOIN page p ON cl.cl_to = p.page_title 
WHERE page_namespace=14 
ORDER BY cl_from
