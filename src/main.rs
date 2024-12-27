use axum::{extract::{DefaultBodyLimit, Multipart, Query}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router};
use connection::establish_connection;
use course_retreival::{get_course_details_from_db, get_courses_from_db, insert_course_link_into_db, insert_course_resource_into_db, CourseResourceUploadFile};
use models::{GetCourseDetailsQuery, GetCoursesQuery, InsertCourseResource};

mod models;
mod schema;
mod faculties;
mod connection;
mod course_retreival;
mod course_initialization;
mod authentication;

use crate::models::ErrorResponse;
use tower_http::cors::{CorsLayer, Any};
use axum::extract::Path;
use chrono::Datelike;

#[derive(serde::Deserialize, serde::Serialize)]
struct InsertCourseLinkRequest { 
    title: String,
    url: String
}

#[tokio::main]
async fn main() {
    // Initialize courses if table is empty
    if let Err(e) = course_initialization::initialize_courses_if_empty().await {
        eprintln!("Failed to initialize courses: {}", e);
    }

    // Initialize token cache
    if let Err(e) = authentication::get_token_cache().await {
        eprintln!("Failed to initialize token cache: {}", e);
    }

    println!("Initialized token cache for buckets");

    let mut app = Router::new()
        .route("/v1/courses", get(get_courses))
        .route("/v1/course_details/:course_id", get(get_course_details))
        .route("/v1/course_resource/:course_id", post(insert_course_resource))
        .route("/v1/course_link/:course_id", post(insert_course_link))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024));

    if dotenvy::var("LOCAL_DEV_DEPLOYMENT").is_ok() {
        println!("Local dev deployment");
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any);
        app = app.layer(cors).layer(DefaultBodyLimit::max(1024 * 1024 * 1024));
    }

    println!("Starting server on port 9093");
    axum::Server::bind(&"0.0.0.0:9093".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_course_link(Path(course_id): Path<String>, Json(payload): Json<InsertCourseLinkRequest> ) -> Result<impl IntoResponse, StatusCode> {
    let conn = &mut establish_connection().unwrap();
    match insert_course_link_into_db(conn, payload.title, payload.url, course_id) {
        Ok(_) => Ok(StatusCode::OK.into_response()),
        Err(e) => Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })).into_response())
    }
}

pub async fn insert_course_resource(
    Path(course_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    // Get the JSON part first
    let mut payload: Option<InsertCourseResource> = None;
    let mut files: Vec<CourseResourceUploadFile> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "metadata" {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            payload = Some(serde_json::from_slice(&data).map_err(|_| StatusCode::BAD_REQUEST)?);
        } else if name == "files" {
            let file_name = field.file_name().ok_or(StatusCode::BAD_REQUEST)?.to_string();
            println!("Entering file {}", file_name);
            let file_data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            println!("Got data of file {}", file_name);
            let file_data_vec = file_data.to_vec();
            files.push(CourseResourceUploadFile { filename: file_name.clone(), data: file_data_vec });
            println!("File uploaded: {}", file_name);
        }
    }

    if files.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "User must upload at least one file".to_string() 
        })).into_response());
    }

    let payload = match payload {
        Some(p) => p,
        None => return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Payload is required".to_string() 
        })).into_response())
    };

    println!("{:?}", payload);


    let sem = match payload.semester.to_lowercase().as_str() {
        "first" => "First".to_string(),
        "second" => "Second".to_string(),
        "summer" => "Summer".to_string(),
        _ => return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Invalid semester".to_string() 
        })).into_response())
    };

    if payload.title.replace(" ", "").is_empty() || payload.course_id.replace(" ", "").is_empty() {
        return Ok((StatusCode::BAD_REQUEST, Json(ErrorResponse { 
            error: "Title / course id can't be empty".to_string() 
        })).into_response());
    }

    println!("Reached 2");

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
        files
    ).await {
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