use diesel::prelude::*;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use serde_json::from_reader;
use crate::connection::establish_connection;
use crate::schema::courses;

#[derive(Deserialize)]
struct CourseJson {
    faculty: i16,
    id: String,
    name: String
}

#[derive(Insertable)]
#[diesel(table_name = courses)]
struct NewCourse {
    course_id: String,
    course_name: String,
    course_faculty: i16,
}

pub async fn initialize_courses_if_empty() -> Result<(), Box<dyn std::error::Error>> {
    let conn = &mut establish_connection()?;

    // Check if courses table is empty
    use crate::schema::courses::dsl::*;
    let count: i64 = courses.count().get_result(conn)?;
    
    if count != 0 {
        return Ok(());
    }

    let json_file_path = dotenvy::var("COURSES_JSON_PATH")?;
    let file = File::open(json_file_path)?;
    let reader = BufReader::new(file);
    let courses_data: Vec<CourseJson> = from_reader(reader)?;

    for course_json in courses_data {
        let new_course = NewCourse {
            course_id: course_json.id,
            course_name: course_json.name,
            course_faculty: course_json.faculty
        };
        
        diesel::insert_into(courses)
            .values(&new_course)
            .execute(conn)?;
    }

    Ok(())
} 