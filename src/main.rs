mod course_retreival;
mod course_initialization;
use axum::{http::StatusCode, extract::Query, response::IntoResponse, routing::{get, post}, Router, Json, extract::Multipart};
use connection::establish_connection;
use course_retreival::{get_course_details_from_db, get_courses_from_db, insert_course_resource_into_db};
use models::{GetCourseDetailsQuery, GetCoursesQuery, InsertCourseResource};
mod models;
mod schema;
mod faculties;
mod connection;
use crate::models::ErrorResponse;
use tower_http::cors::{CorsLayer, Any};
use axum::extract::Path;
use chrono::Datelike;

#[tokio::main]
async fn main() {
    // Initialize courses if table is empty
    if let Err(e) = course_initialization::initialize_courses_if_empty().await {
        eprintln!("Failed to initialize courses: {}", e);
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/v1/courses", get(get_courses))
        .route("/v1/course_details/:course_id", get(get_course_details))
        .route("/v1/course_resource/:course_id", post(insert_course_resource))
        .layer(cors);

    axum::Server::bind(&"0.0.0.0:9093".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn insert_course_resource(
    Path(course_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    // Get the JSON part first
    let mut payload: Option<InsertCourseResource> = None;
    let mut files: Vec<(String, axum::body::Bytes)> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "metadata" {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            payload = Some(serde_json::from_slice(&data).map_err(|_| StatusCode::BAD_REQUEST)?);
        } else if name == "files" {
            println!("{:?}", field.headers());
            let file_name = field.file_name().ok_or(StatusCode::BAD_REQUEST)?.to_string();
            let file_data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            files.push((file_name, file_data));
        }
    }

    let payload = payload.ok_or(StatusCode::BAD_REQUEST)?;
    println!("{:?}", payload);

    let sem = match payload.semester.to_lowercase().as_str() {
        "fall" => "Fall".to_string(),
        "spring" => "Spring".to_string(),
        "summer" => "Summer".to_string(),
        _ => return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Invalid semester".to_string() 
        })).into_response())
    };

    if payload.title.replace(" ", "").is_empty() || payload.course_id.replace(" ", "").is_empty() {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Title and course id can't be empty".to_string() 
        })).into_response());
    }

    if payload.resource_type != 0 && payload.resource_type != 1 {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Invalid resource type (Must be either 0 for Notes, or 1 for Exams)".to_string() 
        })).into_response());
    }

    if payload.academic_year > chrono::Utc::now().year() {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Academic year can't be greater than the current year".to_string() 
        })).into_response());
    }

    if payload.academic_year < 2000 {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Academic year can't be less than 2000".to_string() 
        })).into_response());
    }

    let conn = &mut establish_connection().unwrap();
    match insert_course_resource_into_db(
        conn, 
        payload.title, 
        payload.subtitle, 
        course_id, 
        payload.resource_type, 
        sem, 
        payload.academic_year, 
        payload.issolved,
        // files,
    ) {
        Ok(resource) => Ok(Json(resource).into_response()),
        Err(e) => Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { 
            error: e.to_string() 
        })).into_response())
    }
}

async fn get_course_details(course_id: Path<String>, query: Query<GetCourseDetailsQuery>) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut establish_connection().unwrap();
    let id = course_id.0.clone();
    let resource_type = query.0.resource_type;
    if resource_type != 0 && resource_type != 1 {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid resource type (Must be either 0 for Notes, or 1 for Exams)".to_string() })).into_response());
    }

    let course_details = get_course_details_from_db(conn, id, resource_type);
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