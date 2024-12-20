// @generated automatically by Diesel CLI.

diesel::table! {
    courses (course_id) {
        course_id -> Varchar,
        course_name -> Varchar,
        course_faculty -> Int2,
    }
}
