use crate::schema::{courses, course_resources, course_resource_files, course_resource_links};
use chrono::Utc;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Queryable, Selectable, Serialize, Deserialize, Insertable)]
#[diesel(table_name = courses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Course {    
    pub course_id: String, /* CS116 */
    pub course_name: String, /* Computing Fundamentals */
    pub course_faculty: i16
}

#[derive(Queryable, Selectable, Serialize, Deserialize, Insertable)]
#[diesel(table_name = course_resources)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CourseResource {
    pub title: String,
    pub subtitle: Option<String>,

    pub resource_id: Uuid,
    pub course_id: String,

    pub resource_type: i16, /* 0 = Notes, 1 = Exams */

    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub dateuploaded: chrono::DateTime<Utc>,
    
    pub semester: String,
    pub academic_year: i32,
    pub issolved: bool
}

#[derive(Deserialize, Serialize, Queryable, Debug)]
pub struct InsertCourseResource {
    pub title: String,
    pub subtitle: Option<String>,
    pub course_id: String,
    pub resource_type: i16,
    pub semester: String,
    pub academic_year: i32,
    pub issolved: bool,
}

#[derive(Deserialize, Serialize, Queryable)]
pub struct InsertCourseResourceFile {
    pub file_name: String,
}

#[derive(Queryable, Selectable, Serialize, Insertable)]
#[diesel(table_name = course_resource_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CourseResourceFile {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_url: String,
    pub resource_id: Uuid
}

#[derive(Serialize)]
pub struct GetCoursesResponse { 
    pub courses: Vec<Course>,
    pub total_courses: i64
}

#[derive(Serialize)]
pub struct ErrorResponse {
   pub  error: String,
}

#[derive(Deserialize)]
pub struct GetCoursesQuery { 
    pub faculty: Option<i16>,
    pub search: Option<String>, /* searchTerm */
    pub page: Option<i64>
}

#[derive(Deserialize)]
pub struct GetCourseDetailsQuery {
    pub resource_type: i16
}

#[derive(Serialize)]
pub struct CourseDetailsResourceResponse {
    pub resource_info: CourseResource,
    pub files: Vec<CourseResourceFile>
}

#[derive(Serialize)]
pub struct CourseDetailsLinkResponse {
    pub title: String,
    pub url: String
}

#[derive(Serialize)]
pub struct CourseDetails {
    pub metadata: Course,
    pub resources: Vec<CourseDetailsResourceResponse>,
    pub links: Vec<CourseDetailsLinkResponse>
}

#[derive(Deserialize, Serialize, Insertable, Queryable, Debug)]
#[diesel(table_name = course_resource_links)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CourseResourceLink {
    pub link_id: Uuid,
    pub link_title: String,
    pub link_url: String,
    pub course_id: String
}