use crate::models::AppState;
use crate::services::composite::{PageViewsService, ServiceError as CompositeServiceError};
use crate::services::core::{
    ArticleService, CategoryService, CoreServiceError, PageViewService, QidService,
};
use chrono::NaiveDate;
use std::sync::Arc;
use tonic::{Request, Response, Status};

// Include the generated proto code
pub mod topictrend_proto {
    tonic::include_proto!("topictrend");
}

use topictrend_proto::{
    ArticleCategoriesRequest,
    ArticleCategoriesResponse,

    ArticleTrendRequest,
    ArticleTrendResponse,
    ArticleViewsRequest,
    ArticleViewsResponse,
    CategoryArticlesRequest,
    CategoryArticlesResponse,
    // Composite messages (legacy)
    CategoryTrendRequest,
    CategoryTrendResponse,
    // Raw data messages
    CategoryViewsRequest,
    CategoryViewsResponse,
    // Graph messages
    ChildCategoriesRequest,
    ChildCategoriesResponse,
    // Data structures
    DailyViews,
    ParentCategoriesRequest,
    ParentCategoriesResponse,
    QidByTitleRequest,
    QidByTitleResponse,

    QidsByTitlesRequest,
    QidsByTitlesResponse,
    RawArticleViews,
    RawCategoryViews,
    SubCategoryRequest,
    SubCategoryResponse,

    TitleByQidRequest,
    TitleByQidResponse,
    // Metadata messages
    TitlesByQidsRequest,
    TitlesByQidsResponse,
    TopArticle,
    TopArticlesRawRequest,
    TopArticlesRawResponse,

    TopCategoriesRawRequest,
    TopCategoriesRawResponse,
    TopCategoriesRequest,
    TopCategoriesResponse,
    TopCategory,

    ValidateArticleRequest,
    // Validation messages
    ValidateCategoryRequest,
    ValidationResponse,

    // Service trait
    topic_trend_service_server::TopicTrendService,
};

pub struct TopicTrendGrpcService {
    state: Arc<AppState>,
}

impl TopicTrendGrpcService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl From<CoreServiceError> for Status {
    fn from(err: CoreServiceError) -> Self {
        match err {
            CoreServiceError::DatabaseError(e) => {
                Status::internal(format!("Database error: {}", e))
            }
            CoreServiceError::EngineError(e) => Status::internal(format!("Engine error: {}", e)),
            CoreServiceError::NotFound => Status::not_found("Resource not found"),
            CoreServiceError::InternalError(e) => {
                Status::internal(format!("Internal error: {}", e))
            }
        }
    }
}

impl From<CompositeServiceError> for Status {
    fn from(err: CompositeServiceError) -> Self {
        match err {
            CompositeServiceError::CoreError(core_err) => core_err.into(),
        }
    }
}

fn parse_date(date_str: &str) -> Result<NaiveDate, Status> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| Status::invalid_argument("Invalid date format, expected YYYY-MM-DD"))
}

#[tonic::async_trait]
impl TopicTrendService for TopicTrendGrpcService {
    // Raw data endpoints
    async fn get_category_views(
        &self,
        request: Request<CategoryViewsRequest>,
    ) -> Result<Response<CategoryViewsResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;
        let depth = req.depth.unwrap_or(0);

        let views = PageViewService::get_raw_category_views(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
            start_date,
            end_date,
            depth,
        )
        .await
        .map_err(Status::from)?;

        let daily_views: Vec<DailyViews> = views
            .into_iter()
            .map(|(date, views)| DailyViews {
                date: date.to_string(),
                views,
            })
            .collect();

        Ok(Response::new(CategoryViewsResponse { views: daily_views }))
    }

    async fn get_article_views(
        &self,
        request: Request<ArticleViewsRequest>,
    ) -> Result<Response<ArticleViewsResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;

        let views = PageViewService::get_raw_article_views(
            Arc::clone(&self.state),
            &req.wiki,
            req.article_qid,
            start_date,
            end_date,
        )
        .await
        .map_err(Status::from)?;

        let daily_views: Vec<DailyViews> = views
            .into_iter()
            .map(|(date, views)| DailyViews {
                date: date.to_string(),
                views,
            })
            .collect();

        Ok(Response::new(ArticleViewsResponse { views: daily_views }))
    }

    async fn get_top_categories_raw(
        &self,
        request: Request<TopCategoriesRawRequest>,
    ) -> Result<Response<TopCategoriesRawResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;
        let limit = req.limit.unwrap_or(10) as usize;

        let categories = PageViewService::get_top_categories_raw(
            Arc::clone(&self.state),
            &req.wiki,
            start_date,
            end_date,
            limit,
        )
        .await
        .map_err(Status::from)?;

        let raw_categories: Vec<RawCategoryViews> = categories
            .into_iter()
            .map(|cat| {
                let top_articles: Vec<RawArticleViews> = cat
                    .top_articles
                    .into_iter()
                    .map(|art| RawArticleViews {
                        article_qid: art.article_qid,
                        total_views: art.total_views,
                    })
                    .collect();

                RawCategoryViews {
                    category_qid: cat.category_qid,
                    total_views: cat.total_views,
                    top_articles,
                }
            })
            .collect();

        Ok(Response::new(TopCategoriesRawResponse {
            categories: raw_categories,
        }))
    }

    async fn get_top_articles_raw(
        &self,
        request: Request<TopArticlesRawRequest>,
    ) -> Result<Response<TopArticlesRawResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;
        let depth = req.depth.unwrap_or(0);
        let limit = req.limit.unwrap_or(10) as usize;

        let articles = PageViewService::get_top_articles_raw(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
            start_date,
            end_date,
            depth,
            limit,
        )
        .await
        .map_err(Status::from)?;

        let raw_articles: Vec<RawArticleViews> = articles
            .into_iter()
            .map(|art| RawArticleViews {
                article_qid: art.article_qid,
                total_views: art.total_views,
            })
            .collect();

        Ok(Response::new(TopArticlesRawResponse {
            articles: raw_articles,
        }))
    }

    // Metadata endpoints
    async fn get_titles_by_qids(
        &self,
        request: Request<TitlesByQidsRequest>,
    ) -> Result<Response<TitlesByQidsResponse>, Status> {
        let req = request.into_inner();

        let titles = QidService::get_titles_by_qids(Arc::clone(&self.state), &req.wiki, req.qids)
            .await
            .map_err(Status::from)?;

        Ok(Response::new(TitlesByQidsResponse { titles }))
    }

    async fn get_qids_by_titles(
        &self,
        request: Request<QidsByTitlesRequest>,
    ) -> Result<Response<QidsByTitlesResponse>, Status> {
        let req = request.into_inner();
        let namespace = req.namespace.unwrap_or(0) as i8;

        let qids = QidService::get_qids_by_titles(
            Arc::clone(&self.state),
            &req.wiki,
            req.titles,
            namespace,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(QidsByTitlesResponse { qids }))
    }

    async fn get_title_by_qid(
        &self,
        request: Request<TitleByQidRequest>,
    ) -> Result<Response<TitleByQidResponse>, Status> {
        let req = request.into_inner();

        let title = QidService::get_title_by_qid(Arc::clone(&self.state), &req.wiki, req.qid)
            .await
            .map_err(Status::from)?;

        Ok(Response::new(TitleByQidResponse { title }))
    }

    async fn get_qid_by_title(
        &self,
        request: Request<QidByTitleRequest>,
    ) -> Result<Response<QidByTitleResponse>, Status> {
        let req = request.into_inner();
        let namespace = req.namespace.unwrap_or(0) as i8;

        let qid =
            QidService::get_qid_by_title(Arc::clone(&self.state), &req.wiki, &req.title, namespace)
                .await
                .map_err(Status::from)?;

        Ok(Response::new(QidByTitleResponse { qid }))
    }

    // Graph endpoints
    async fn get_child_categories(
        &self,
        request: Request<ChildCategoriesRequest>,
    ) -> Result<Response<ChildCategoriesResponse>, Status> {
        let req = request.into_inner();

        let category_qids = CategoryService::get_child_categories(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(ChildCategoriesResponse { category_qids }))
    }

    async fn get_parent_categories(
        &self,
        request: Request<ParentCategoriesRequest>,
    ) -> Result<Response<ParentCategoriesResponse>, Status> {
        let req = request.into_inner();

        let category_qids = CategoryService::get_parent_categories(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(ParentCategoriesResponse { category_qids }))
    }

    async fn get_category_articles(
        &self,
        request: Request<CategoryArticlesRequest>,
    ) -> Result<Response<CategoryArticlesResponse>, Status> {
        let req = request.into_inner();
        let depth = req.depth.unwrap_or(0);

        let article_qids = CategoryService::get_category_articles(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
            depth,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(CategoryArticlesResponse { article_qids }))
    }

    async fn get_article_categories(
        &self,
        request: Request<ArticleCategoriesRequest>,
    ) -> Result<Response<ArticleCategoriesResponse>, Status> {
        let req = request.into_inner();

        let category_qids = ArticleService::get_article_categories(
            Arc::clone(&self.state),
            &req.wiki,
            req.article_qid,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(ArticleCategoriesResponse { category_qids }))
    }

    // Validation endpoints
    async fn validate_category_exists(
        &self,
        request: Request<ValidateCategoryRequest>,
    ) -> Result<Response<ValidationResponse>, Status> {
        let req = request.into_inner();

        let exists = CategoryService::validate_category_exists(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(ValidationResponse { exists }))
    }

    async fn validate_article_exists(
        &self,
        request: Request<ValidateArticleRequest>,
    ) -> Result<Response<ValidationResponse>, Status> {
        let req = request.into_inner();

        let exists = ArticleService::validate_article_exists(
            Arc::clone(&self.state),
            &req.wiki,
            req.article_qid,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(ValidationResponse { exists }))
    }

    // Legacy composite endpoints (for backward compatibility)
    async fn get_category_pageviews(
        &self,
        request: Request<CategoryTrendRequest>,
    ) -> Result<Response<CategoryTrendResponse>, Status> {
        let req = request.into_inner();

        let start_date = if !req.start_date.as_ref().unwrap_or(&String::new()).is_empty() {
            Some(parse_date(req.start_date.as_ref().unwrap())?)
        } else {
            None
        };

        let end_date = if !req.end_date.as_ref().unwrap_or(&String::new()).is_empty() {
            Some(parse_date(req.end_date.as_ref().unwrap())?)
        } else {
            None
        };

        let result = PageViewsService::get_category_trend(
            Arc::clone(&self.state),
            &req.wiki,
            &req.category,
            req.category_qid,
            req.depth,
            start_date,
            end_date,
        )
        .await
        .map_err(Status::from)?;

        let daily_views: Vec<DailyViews> = result
            .views
            .into_iter()
            .map(|(date, views)| DailyViews {
                date: date.to_string(),
                views,
            })
            .collect();

        let top_articles: Vec<TopArticle> = result
            .top_articles
            .into_iter()
            .map(|art| TopArticle {
                qid: art.qid,
                title: art.title,
                views: art.views,
            })
            .collect();

        let response = CategoryTrendResponse {
            qid: result.qid,
            title: result.title,
            views: daily_views,
            top_articles,
        };

        Ok(Response::new(response))
    }

    async fn get_article_pageviews(
        &self,
        request: Request<ArticleTrendRequest>,
    ) -> Result<Response<ArticleTrendResponse>, Status> {
        let req = request.into_inner();

        let start_date = if !req.start_date.as_ref().unwrap_or(&String::new()).is_empty() {
            Some(parse_date(req.start_date.as_ref().unwrap())?)
        } else {
            None
        };

        let end_date = if !req.end_date.as_ref().unwrap_or(&String::new()).is_empty() {
            Some(parse_date(req.end_date.as_ref().unwrap())?)
        } else {
            None
        };

        let result = PageViewsService::get_article_trend(
            Arc::clone(&self.state),
            &req.wiki,
            &req.article,
            req.article_qid,
            start_date,
            end_date,
        )
        .await
        .map_err(Status::from)?;

        let daily_views: Vec<DailyViews> = result
            .views
            .into_iter()
            .map(|(date, views)| DailyViews {
                date: date.to_string(),
                views,
            })
            .collect();

        let response = ArticleTrendResponse {
            qid: result.qid,
            title: result.title,
            views: daily_views,
        };

        Ok(Response::new(response))
    }

    async fn get_top_categories(
        &self,
        request: Request<TopCategoriesRequest>,
    ) -> Result<Response<TopCategoriesResponse>, Status> {
        let req = request.into_inner();

        let start_date = if !req.start_date.as_ref().unwrap_or(&String::new()).is_empty() {
            Some(parse_date(req.start_date.as_ref().unwrap())?)
        } else {
            None
        };

        let end_date = if !req.end_date.as_ref().unwrap_or(&String::new()).is_empty() {
            Some(parse_date(req.end_date.as_ref().unwrap())?)
        } else {
            None
        };

        let categories = PageViewsService::get_top_categories(
            Arc::clone(&self.state),
            &req.wiki,
            start_date,
            end_date,
            req.top_n,
        )
        .await
        .map_err(Status::from)?;

        let grpc_categories: Vec<TopCategory> = categories
            .into_iter()
            .map(|cat| {
                let top_articles: Vec<TopArticle> = cat
                    .top_articles
                    .into_iter()
                    .map(|art| TopArticle {
                        qid: art.qid,
                        title: art.title,
                        views: art.views,
                    })
                    .collect();

                TopCategory {
                    qid: cat.qid,
                    title: cat.title,
                    views: cat.views,
                    top_articles,
                }
            })
            .collect();

        let response = TopCategoriesResponse {
            categories: grpc_categories,
        };

        Ok(Response::new(response))
    }

    async fn get_sub_categories(
        &self,
        request: Request<SubCategoryRequest>,
    ) -> Result<Response<SubCategoryResponse>, Status> {
        let req = request.into_inner();

        let categories = PageViewsService::get_sub_categories(
            Arc::clone(&self.state),
            &req.wiki,
            &req.category,
            req.category_qid,
        )
        .await
        .map_err(Status::from)?;

        let response = SubCategoryResponse { categories };

        Ok(Response::new(response))
    }
}
