use core::error;

use axum::{http::StatusCode, extract::Query, response::{IntoResponse, Response}, routing::{get, /*post*/}, Router, Json};
use connection::{establish_connection, get_course_details_from_db, get_courses_from_db};
use diesel::{deserialize};
use models::{Course, GetCoursesQuery};
use serde::Deserialize;
mod models;
mod schema;
mod faculties;
mod connection;
use crate::models::ErrorResponse;
use tower_http::cors::{CorsLayer, Any};
use axum::extract::Path;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/v1/courses", get(get_courses))
        .route("/v1/course_details/:course_id", get(get_course_details))
        .layer(cors);

    axum::Server::bind(&"0.0.0.0:9093".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_course_details(course_id: Path<String>) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut establish_connection().unwrap();
    let id = course_id.0.clone();
    let course_details = get_course_details_from_db(conn, id);
    match course_details {
        Ok(course_details) => {
            Ok(Json(course_details).into_response())
        },
        Err(e) => {
            if e == diesel::result::Error::NotFound {
                return Ok((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Course with id {} not found", course_id.0) })).into_response());
            }

            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response())
        }
    }
}

async fn get_courses(query: Query<GetCoursesQuery>) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut establish_connection().unwrap();
    let get_courses_q = query.0;

    match get_courses_from_db(conn, get_courses_q.faculty, get_courses_q.search, get_courses_q.page) { 
        Ok(courses) => { 
            Ok(Json(courses).into_response())
        },
        Err(e) => {       
            let error_response = ErrorResponse { error: e.to_string() };
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response())
        }
    }
}