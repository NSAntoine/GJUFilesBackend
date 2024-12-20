use core::fmt;
use std::arch::is_aarch64_feature_detected;
use std::borrow::{Borrow, BorrowMut};
use std::env;

use diesel::dsl::{count_star, max};
use diesel::{ExpressionMethods, PgTextExpressionMethods, BoolExpressionMethods, TextExpressionMethods};
use dotenvy::dotenv;
use diesel::{Connection, PgConnection, QueryDsl, RunQueryDsl};
use crate::schema;
use crate::models::{Course, CourseDetails, GetCoursesQuery, GetCoursesResponse};

pub fn establish_connection() -> Result<PgConnection, diesel::result::ConnectionError> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")
    .map_err(
        |_| diesel::result::ConnectionError::BadConnection("DATABASE_URL not set".into())
    )?;
    PgConnection::establish(&db_url)
}

pub fn get_course_details_from_db(conn: &mut PgConnection, course_id: String) -> Result<CourseDetails, diesel::result::Error> {
    use schema::courses;
    let query = courses::table.filter(courses::course_id.eq(course_id.to_uppercase()));
    if let Ok(course) = query.first(conn) {
        return Ok(CourseDetails { metadata: course });
    }
    return Err(diesel::result::Error::NotFound);
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

    if let Some(fac) = faculty { 
        query = query.filter(courses::course_faculty.eq(fac));
    }

    // Create a separate count query with the same conditions
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

    let total_count = count_query.select(count_star()).get_result::<i64>(conn)?;

    query = query.limit(limit);

    let offset_page: i64 = sanitize_page_input(page);
    query = query
        .offset((offset_page - 1) * limit)
        .order(courses::course_id.asc());
    
    match query.load(conn) { 
        Ok(vecs) => {
            return Ok(GetCoursesResponse { courses: vecs, total_courses: total_count })
        }
        Err(error) => Err(error)
    }
}