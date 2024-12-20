use crate::schema::{courses, course_resources};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = courses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Course {    
    pub course_id: String, /* CS116 */
    pub course_name: String, /* Computing Fundamentals */
    pub course_faculty: i16
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = course_resources)]
pub struct CourseResource {
    pub resource_id: String,
    pub title: String,
    pub subtitle: String,
    pub resource_type: i16,
    pub date_uploaded: String,
    pub semester: String,
    pub academic_year: i32,
    pub is_solved: bool
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

#[derive(Serialize)]
pub struct CourseDetails {
    pub metadata: Course,
}