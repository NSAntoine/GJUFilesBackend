use diesel::dsl::count_star;
use diesel::{BoolExpressionMethods, ExpressionMethods, PgTextExpressionMethods, SelectableHelper};
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use uuid::Uuid;
use crate::authentication::get_token_cache;
use crate::schema::{self, course_resources};
use crate::models::{CourseDetails, CourseDetailsResourceResponse, CourseResource, CourseResourceFile, GetCoursesResponse};
use reqwest::Client;

pub struct CourseResourceUploadFile {
    pub filename: String,
    pub data: Vec<u8>
}

pub fn get_course_details_from_db(conn: &mut PgConnection, course_id: String, resource_type: i16) -> Result<CourseDetails, diesel::result::Error> {
    use schema::courses;
    let query = courses::table.filter(courses::course_id.eq(course_id.to_uppercase()));
    if let Ok(course) = query.first(conn) {
        return Ok(CourseDetails { metadata: course, resources: get_course_resources_from_db(conn, course_id, resource_type)? });
    }
    return Err(diesel::result::Error::NotFound);
}

fn get_course_resources_from_db(conn: &mut PgConnection, course_id: String, resource_type: i16) -> Result<Vec<CourseDetailsResourceResponse>, diesel::result::Error> {
    use schema::course_resources;
    let query = course_resources::table.filter(course_resources::course_id.eq(course_id.to_uppercase()).and(course_resources::resource_type.eq(resource_type)));
    if let Ok(resources) = query.load::<CourseResource>(conn) {
        let mut resources_with_files: Vec<CourseDetailsResourceResponse> = Vec::new();
        for resource in resources {
            if let Ok(files) = get_course_resource_files_from_db(conn, resource.resource_id) {
                resources_with_files.push(CourseDetailsResourceResponse { resource_info: resource, files });
            }
        }

        return Ok(resources_with_files);
    }
    return Err(diesel::result::Error::NotFound);
}

fn get_course_resource_files_from_db(conn: &mut PgConnection, resource_id: Uuid) -> Result<Vec<CourseResourceFile>, diesel::result::Error> {
    use schema::course_resource_files;
    let query = course_resource_files::table.filter(course_resource_files::resource_id.eq(resource_id));
    if let Ok(files) = query.load(conn) {
        return Ok(files);
    }
    return Err(diesel::result::Error::NotFound);
}

pub async fn insert_course_resource_into_db(conn: &mut PgConnection, title: String, subtitle: Option<String>, course_id: String, resource_type: i16, semester: String, academic_year: i32, is_solved: bool, files: Vec<CourseResourceUploadFile>) -> Result<CourseResource, diesel::result::Error> {
    let new_resource_id = Uuid::new_v4();
    let bucket_folder_name = format!("course_resources/{}/{}", course_id, new_resource_id);
    // concurrently upload files to bucket
    let bucket_name = "gjufilesresources";
    for file in files {
        let file_in_bucket_name = format!("{}/{}", bucket_folder_name, file.filename);
        let url = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            bucket_name, file_in_bucket_name
        );

        let client = Client::new()
        .post(&url)
        .bearer_auth(get_token_cache().await.unwrap());
    }

    let new_resource = CourseResource {
        title,
        subtitle,
        resource_id: new_resource_id,
        course_id,
        resource_type,
        dateuploaded: chrono::Utc::now(),
        semester,
        academic_year,
        issolved: is_solved
    };

    diesel::insert_into(course_resources::table)
        .values(new_resource)
        .returning(CourseResource::as_returning())
        .get_result(conn)
}

// Sanitize page number input for getting courses
// if the number is null or below 0 
// return 1
// otherwise return the original
fn sanitize_page_input(page: Option<i64>) -> i64 {

    if let Some(pageN) = page {
        if pageN <= 0 {
            return 1;
        }

        return pageN;
    }

    return 1;
}

fn courses_per_page() -> i64 {
    return 12;
}

pub fn get_courses_from_db(conn: &mut PgConnection, faculty: Option<i16>, searchTerm: Option<String>, page: Option<i64>) -> Result<GetCoursesResponse, diesel::result::Error> {
    use schema::courses;
    let limit = courses_per_page();

    let mut query = courses::table.into_boxed();
    if let Some(search) = searchTerm.clone() {
        let formatted_string = format!("%{}%", search.clone());
        query = query.filter(
            courses::course_id.ilike(formatted_string.clone()).or(courses::course_name.ilike(formatted_string.clone()))
        );
    }

    /* 
    Resource: 

    Title (String)
    Subtitle (String, Nullable)

    ResourceID (UUID)
    CourseID (String)

    DateUploaded (DATE)
    Semester (String, could be Fall, Spring, Summer)
    Type (Integer, 0 = Notes, 1 = Exams)
    Year (Int)
    isSolved (Boolean)
    */

    // TODO: Refactor query and count query to use a macro
    if let Some(fac) = faculty { 
        query = query.filter(courses::course_faculty.eq(fac));
    }

    // Create a separate count query with the same conditions, but without the limit (for pagination)
    let mut count_query = courses::table.into_boxed();
    
    if let Some(search) = searchTerm {
        let formatted_string = format!("%{}%", search);
        count_query = count_query.filter(
            courses::course_id.ilike(formatted_string.clone())
            .or(courses::course_name.ilike(formatted_string))
        )
    }

    if let Some(fac) = faculty {
        count_query = count_query.filter(courses::course_faculty.eq(fac))
    }

    // the total count without the limit that's used for pagination
    let total_count = count_query.select(count_star()).get_result::<i64>(conn)?;

    query = query.limit(limit);

    let offset_page: i64 = sanitize_page_input(page);
    query = query
        .offset((offset_page - 1) * limit)
        .order(courses::course_id.asc());
    
    return Ok(GetCoursesResponse { courses: query.load(conn)?, total_courses: total_count });
}