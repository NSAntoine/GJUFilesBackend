# GJU Files Backend
This is the backend used for https://gjufiles.com, a website made for GJU Students to share resource with each other such as notes & past exams

# Environment Variables
- `DATABASE_URL`: URL for Postgres database to use in the backend
- `COURSES_JSON_PATH`: Path of JSON file that includes the courses to show in the backend (by default this is included in `src/data/Courses.json`)
- `GOOGLE_APPLICATION_CREDENTIALS`: Path to JSON file for your Google Cloud Application Default Credentials
- `LOCAL_DEV_DEPLOYMENT`: Set this to 1 if you're testing the frontend on localhost to get past CORS

# Build & Run
```
cargo run --release
```

# Planned Features
- Rating Courses based off difficulty, etc
