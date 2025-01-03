use diesel::dsl::count_star;
use diesel::{BoolExpressionMethods, ExpressionMethods, PgTextExpressionMethods, SelectableHelper};
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use uuid::Uuid;
use crate::authentication::get_token_cache;
use crate::schema::{self, course_resource_links, course_resources};
use crate::models::{CourseDetails, CourseDetailsLinkResponse, CourseDetailsResourceResponse, CourseResource, CourseResourceFile, CourseResourceLink, GetCoursesResponse};
use reqwest::Client;

/* Don't make this public. it's what the gcloud api returns when you upload a file */
#[derive(Debug, serde::Deserialize)]
struct __UploadResourceFileResponse {
    pub mediaLink: String
}

pub struct CourseResourceUploadFile {
    pub filename: String,
    pub data: Vec<u8>
}

pub fn get_course_details_from_db(conn: &mut PgConnection, course_id: String, resource_type: i16) -> Result<CourseDetails, diesel::result::Error> {
    use schema::courses;
    let query = courses::table.filter(courses::course_id.eq(course_id.to_uppercase()));

    use schema::course_resources;
    let course_resources_query = course_resources::table.filter(course_resources::course_id.eq(course_id.to_uppercase()));
    let no_notes = course_resources_query.clone().filter(course_resources::resource_type.eq(0)).select(count_star()).get_result::<i64>(conn)?;
    let no_exams = course_resources_query.filter(course_resources::resource_type.eq(1)).select(count_star()).get_result::<i64>(conn)?;

    if let Ok(course) = query.first(conn) {
        return Ok(CourseDetails { metadata: course, resources: get_course_resources_from_db(conn, course_id.clone(), resource_type)?, links: get_course_links_from_db(conn, course_id)?, no_notes, no_exams });
    }
    return Err(diesel::result::Error::NotFound);
}

fn get_course_links_from_db(conn: &mut PgConnection, course_id: String) -> Result<Vec<CourseDetailsLinkResponse>, diesel::result::Error> { 
    use schema::course_resource_links;
    let query = course_resource_links::table.filter(course_resource_links::course_id.eq(course_id.to_uppercase()));
    let links_from_db = query.load::<CourseResourceLink>(conn)?;
    // We load a CourseResourceLink and then return a CourseDetailsLinkResponse
    // but why??
    // well because I don't wanna return useless info with each link like the courseID and linkID
    // like u already know the courseID if ur requesting the link for the course here...
    let mut links_to_return: Vec<CourseDetailsLinkResponse> = Vec::new();
    for link in links_from_db { 
        links_to_return.push(CourseDetailsLinkResponse { title: link.link_title, url: link.link_url });
    }

    return Ok(links_to_return);
}

pub fn insert_course_link_into_db(conn: &mut PgConnection, link_title: String, link_url: String, course_id: String) -> Result<CourseResourceLink, diesel::result::Error> { 
    let link_uuid = Uuid::new_v4();
    let db_resource_to_insert = CourseResourceLink { 
        course_id,
        link_id: link_uuid,
        link_title,
        link_url
    };
    
    diesel::insert_into(course_resource_links::table)
        .values(db_resource_to_insert)
        .get_result::<CourseResourceLink>(conn)
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

fn sanitize_file_name_to_upload(file_name: String) -> String {
    return file_name.replace(" ", "_")
    .replace("#", "_")
    .replace("'", "_")
    .replace("\"", "_")
    .replace(":", "_")
    .replace(";", "_")
    .replace("|", "_");
}

fn file_content_type(file_name: String) -> String {
    let extension = file_name.split('.').last().unwrap();
    return match extension {
        "pdf" => "application/pdf".to_string(),
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
        "pptx" => "application/vnd.ms-powerpoint".to_string(),
        "ppt" => "application/vnd.ms-powerpoint".to_string(),
        "jpg" => "image/jpeg".to_string(),
        "jpeg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        "gif" => "image/gif".to_string(),
        _ => "application/octet-stream".to_string()
    };
}

pub async fn insert_course_resource_into_db(conn: &mut PgConnection, title: String, subtitle: Option<String>, course_id: String, resource_type: i16, semester: String, academic_year: i32, is_solved: bool, files: Vec<CourseResourceUploadFile>) -> Result<CourseResource, diesel::result::Error> {
    let new_resource_id = Uuid::new_v4();
    let bucket_folder_name = format!("course_resources/{}/{}", course_id, new_resource_id);
    // concurrently upload files to bucket
    let bucket_name = "gjufilesresources";
    let mut new_resource_files: Vec<CourseResourceFile> = Vec::new();
    for file in files {
        let sanitized_file_name = sanitize_file_name_to_upload(file.filename);
        let file_in_bucket_name = format!("{}/{}", bucket_folder_name, sanitized_file_name);
        let request_url = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            bucket_name, file_in_bucket_name
        );

        let content_type = file_content_type(sanitized_file_name.clone());
        println!("Content type: {}", content_type);
        let response = Client::new()
            .post(&request_url)
            .bearer_auth(get_token_cache().await.unwrap())
            .body(file.data)
            .header("Content-Type", content_type)
            .send()
            .await
            .unwrap();
        // Check that the response is a 200-299 status code 
        // to make sure the file uploaded successfully
        if !(200..=299).contains(&response.status().as_u16()) {
            return Err(diesel::result::Error::NotFound);
        }
        
        let file_url = format!("https://storage.googleapis.com/{}/{}", bucket_name, file_in_bucket_name);
        let file_id = Uuid::new_v4();

        let new_resource_file = CourseResourceFile {
            file_id,
            file_name: sanitized_file_name,
            file_url,
            resource_id: new_resource_id
        };

        new_resource_files.push(new_resource_file);
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

    match diesel::insert_into(course_resources::table)
        .values(new_resource)
        .returning(CourseResource::as_returning())
        .get_result(conn) {
            Ok(resource) => {
                if let Err(e) = diesel::insert_into(schema::course_resource_files::table)
                    .values(new_resource_files)
                    .execute(conn) {
                    return Err(e);
                }
                
                return Ok(resource);
            }
            Err(e) => {
                return Err(e);
            }
        }
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