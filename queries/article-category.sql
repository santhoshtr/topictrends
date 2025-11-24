SELECT cl.cl_from AS article_id, 
       p.page_id AS category_page_id 
FROM categorylinks cl 
JOIN linktarget lt ON lt.lt_id = cl.cl_target_id
JOIN page p ON lt.lt_title = p.page_title 
             AND p.page_namespace = 14
WHERE lt.lt_namespace = 14 
  AND cl.cl_type = 'page'

