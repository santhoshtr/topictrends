use crate::models::AppState;
use crate::services::{PageViewsService, ServiceError};
use chrono::NaiveDate;
use std::sync::Arc;
use tonic::{Request, Response, Status};

// Include the generated proto code
pub mod topictrend_proto {
    tonic::include_proto!("topictrend");
}

use topictrend_proto::{
    ArticleTrendRequest, ArticleTrendResponse, CategoryTrendRequest, CategoryTrendResponse,
    DailyViews, SubCategoryRequest, SubCategoryResponse, TopArticle, TopCategoriesRequest,
    TopCategoriesResponse, TopCategory, topic_trend_service_server::TopicTrendService,
};

pub struct TopicTrendGrpcService {
    state: Arc<AppState>,
}

impl TopicTrendGrpcService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl From<ServiceError> for Status {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::DatabaseError(e) => Status::internal(format!("Database error: {}", e)),
            ServiceError::EngineError(e) => Status::internal(format!("Engine error: {}", e)),
            ServiceError::NotFound => Status::not_found("Resource not found"),
            ServiceError::InternalError(e) => Status::internal(format!("Internal error: {}", e)),
        }
    }
}

fn parse_date(date_str: &str) -> Result<NaiveDate, Status> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| Status::invalid_argument("Invalid date format, expected YYYY-MM-DD"))
}

#[tonic::async_trait]
impl TopicTrendService for TopicTrendGrpcService {
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
