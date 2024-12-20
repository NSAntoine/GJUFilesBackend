-- Your SQL goes here
CREATE TABLE courses (
    Course_ID VARCHAR PRIMARY KEY, /* String: course ID, such as CS116 */
    Course_Name VARCHAR NOT NULL, /* String: course name, such as Computing Fundamentals */
    Course_Faculty SMALLINT NOT NULL /* Number: Course Faculty, mapping is defined in the Swift files. */
)

/* A Course Resource can either be a resource for exams or just notes, can have multiple files for a single resource */
CREATE TABLE course_resources (
    Title VARCHAR NOT NULL, /* String: Title of the resource */
    Subtitle VARCHAR, /* String: Subtitle of the resource */

    Resource_ID UUID PRIMARY KEY, /* UUID: Resource ID */
    Course_ID VARCHAR NOT NULL, /* String: course ID, such as CS116 */
    Resource_Type SMALLINT NOT NULL, /* 0 = Notes, 1 = Exams */
    FOREIGN KEY (Course_ID) REFERENCES courses(Course_ID)

    DateUploaded DATE NOT NULL, /* Date: Date the resource was uploaded */
    Semester VARCHAR NOT NULL, /* String: Semester the resource was uploaded in */
    Academic_Year INT NOT NULL, /* Integer: Academic Year of the resource(s) */
    isSolved BOOLEAN NOT NULL /* Boolean: Whether the resource has been solved */
)

/* A Course Resource File is a file for a single resource */
CREATE TABLE course_resource_files (
    File_ID UUID PRIMARY KEY, /* UUID: File ID */
    File_Name VARCHAR NOT NULL, /* String: Name of the file */
    Resource_ID UUID NOT NULL, /* UUID: Resource ID */
    FOREIGN KEY (Resource_ID) REFERENCES course_resources(Resource_ID)
)
