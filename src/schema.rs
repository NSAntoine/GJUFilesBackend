// @generated automatically by Diesel CLI.

diesel::table! {
    course_resource_files (file_id) {
        file_id -> Uuid,
        file_name -> Varchar,
        file_url -> Varchar,
        resource_id -> Uuid,
    }
}

diesel::table! {
    course_resources (resource_id) {
        title -> Varchar,
        subtitle -> Nullable<Varchar>,
        resource_id -> Uuid,
        course_id -> Varchar,
        resource_type -> Int2,
        dateuploaded -> Timestamptz,
        semester -> Varchar,
        academic_year -> Int4,
        issolved -> Bool,
    }
}

diesel::table! {
    courses (course_id) {
        course_id -> Varchar,
        course_name -> Varchar,
        course_faculty -> Int2,
    }
}

diesel::joinable!(course_resource_files -> course_resources (resource_id));
diesel::joinable!(course_resources -> courses (course_id));

diesel::allow_tables_to_appear_in_same_query!(
    course_resource_files,
    course_resources,
    courses,
);
