SELECT page_id, 
  CAST(SUBSTRING(pp_value, 2) AS UNSIGNED) as qid ,
  page_title
FROM page 
JOIN page_props 
  ON pp_page =  page_id and pp_propname = 'wikibase_item' 
WHERE page_namespace = 0
  AND page_is_redirect = 0
  AND page_is_new = 0
