SELECT cl_from AS category, page_id AS parent_category
FROM categorylinks
JOIN page ON page_namespace = 14 AND page_title = cl_to 
WHERE cl_type = 'subcat'
