SELECT p.page_id, p.page_title
FROM page p
WHERE p.page_namespace = 14  -- 14 is the namespace for categories
AND p.page_id NOT IN (
    SELECT pp_page 
    FROM page_props 
    WHERE pp_propname = 'hiddencat'
);
