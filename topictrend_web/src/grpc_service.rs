use crate::models::AppState;
use crate::services::composite::DeltaService;
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
    ArticleDeltaItem,
    ArticleDeltaRequest,
    ArticleDeltaResponse,

    ArticleViews,
    ArticleViewsRequest,
    ArticleViewsResponse,
    CategoryArticlesRequest,
    CategoryArticlesResponse,
    CategoryDeltaItem,
    CategoryDeltaRequest,
    CategoryDeltaResponse,
    CategoryViews,
    //  data messages
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

    TitleByQidRequest,
    TitleByQidResponse,
    // Metadata messages
    TitlesByQidsRequest,
    TitlesByQidsResponse,
    TopArticlesRequest,
    TopArticlesResponse,

    TopCategoriesRequest,
    TopCategoriesResponse,

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

fn parse_date(date_str: &str) -> Result<NaiveDate, Status> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| Status::invalid_argument("Invalid date format, expected YYYY-MM-DD"))
}

#[tonic::async_trait]
impl TopicTrendService for TopicTrendGrpcService {
    //  data endpoints
    async fn get_category_views(
        &self,
        request: Request<CategoryViewsRequest>,
    ) -> Result<Response<CategoryViewsResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;
        let depth = req.depth.unwrap_or(0);

        let views = PageViewService::get_category_views(
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

        let views = PageViewService::get_article_views(
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

    async fn get_top_categories(
        &self,
        request: Request<TopCategoriesRequest>,
    ) -> Result<Response<TopCategoriesResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;
        let limit = req.limit.unwrap_or(10) as usize;

        let categories = PageViewService::get_top_categories(
            Arc::clone(&self.state),
            &req.wiki,
            start_date,
            end_date,
            limit,
        )
        .await
        .map_err(Status::from)?;

        let categories: Vec<CategoryViews> = categories
            .into_iter()
            .map(|cat| {
                let top_articles: Vec<ArticleViews> = cat
                    .top_articles
                    .into_iter()
                    .map(|art| ArticleViews {
                        article_qid: art.article_qid,
                        total_views: art.total_views,
                    })
                    .collect();

                CategoryViews {
                    category_qid: cat.category_qid,
                    total_views: cat.total_views,
                    top_articles,
                }
            })
            .collect();

        Ok(Response::new(TopCategoriesResponse { categories }))
    }

    async fn get_top_articles(
        &self,
        request: Request<TopArticlesRequest>,
    ) -> Result<Response<TopArticlesResponse>, Status> {
        let req = request.into_inner();

        let start_date = parse_date(&req.start_date)?;
        let end_date = parse_date(&req.end_date)?;
        let depth = req.depth.unwrap_or(0);
        let limit = req.limit.unwrap_or(10) as usize;

        let articles = PageViewService::get_top_articles(
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

        let articles: Vec<ArticleViews> = articles
            .into_iter()
            .map(|art| ArticleViews {
                article_qid: art.article_qid,
                total_views: art.total_views,
            })
            .collect();

        Ok(Response::new(TopArticlesResponse { articles }))
    }

    // Delta analysis endpoints
    async fn get_category_delta(
        &self,
        request: Request<CategoryDeltaRequest>,
    ) -> Result<Response<CategoryDeltaResponse>, Status> {
        let req = request.into_inner();

        let baseline_start = parse_date(&req.baseline_start_date)?;
        let baseline_end = parse_date(&req.baseline_end_date)?;
        let impact_start = parse_date(&req.impact_start_date)?;
        let impact_end = parse_date(&req.impact_end_date)?;
        let limit = req.limit.unwrap_or(100) as usize;
        let depth = req.depth.unwrap_or(0);

        let delta_items = DeltaService::get_category_delta(
            Arc::clone(&self.state),
            &req.wiki,
            baseline_start,
            baseline_end,
            impact_start,
            impact_end,
            limit,
            depth,
        )
        .await
        .map_err(Status::from)?;

        let categories: Vec<CategoryDeltaItem> = delta_items
            .into_iter()
            .map(|item| CategoryDeltaItem {
                category_qid: item.category_qid,
                category_title: item.category_title,
                baseline_views: item.baseline_views,
                impact_views: item.impact_views,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            })
            .collect();

        let baseline_period = format!("{} to {}", baseline_start, baseline_end);
        let impact_period = format!("{} to {}", impact_start, impact_end);

        Ok(Response::new(CategoryDeltaResponse {
            categories,
            baseline_period,
            impact_period,
        }))
    }

    async fn get_article_delta(
        &self,
        request: Request<ArticleDeltaRequest>,
    ) -> Result<Response<ArticleDeltaResponse>, Status> {
        let req = request.into_inner();

        let baseline_start = parse_date(&req.baseline_start_date)?;
        let baseline_end = parse_date(&req.baseline_end_date)?;
        let impact_start = parse_date(&req.impact_start_date)?;
        let impact_end = parse_date(&req.impact_end_date)?;
        let limit = req.limit.unwrap_or(100) as usize;
        let depth = req.depth.unwrap_or(0);

        let delta_items = DeltaService::get_article_delta(
            Arc::clone(&self.state),
            &req.wiki,
            req.category_qid,
            baseline_start,
            baseline_end,
            impact_start,
            impact_end,
            limit,
            depth,
        )
        .await
        .map_err(Status::from)?;

        let articles: Vec<ArticleDeltaItem> = delta_items
            .into_iter()
            .map(|item| ArticleDeltaItem {
                article_qid: item.article_qid,
                article_title: item.article_title,
                baseline_views: item.baseline_views,
                impact_views: item.impact_views,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            })
            .collect();

        // Get category title
        let category_title =
            QidService::get_title_by_qid(Arc::clone(&self.state), &req.wiki, req.category_qid)
                .await
                .unwrap_or_else(|_| format!("Q{}", req.category_qid));

        let baseline_period = format!("{} to {}", baseline_start, baseline_end);
        let impact_period = format!("{} to {}", impact_start, impact_end);

        Ok(Response::new(ArticleDeltaResponse {
            articles,
            category_qid: req.category_qid,
            category_title,
            baseline_period,
            impact_period,
        }))
    }

    // Metadata endpoints
    async fn get_titles_by_qids(
        &self,
        request: Request<TitlesByQidsRequest>,
    ) -> Result<Response<TitlesByQidsResponse>, Status> {
        let req = request.into_inner();

        let titles = QidService::get_titles_by_qids(Arc::clone(&self.state), &req.wiki, &req.qids)
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
}
