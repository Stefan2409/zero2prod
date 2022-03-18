use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use tokio::runtime::Runtime;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::run;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // This is a hacky workaround to calling async code from this sync Drop trait to clean up.
        let (tx, rx) = std::sync::mpsc::channel();
        let db_name = self.db_name.clone();

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let config = get_configuration().expect("Failed to read configuration");
                let mut conn =
                    PgConnection::connect(&config.database.connection_string_without_db())
                        .await
                        .expect("Failed to connect to Postgres");

                // Kick off open sessions (from the spawned app). This could be replaced
                // by WITH (FORCE) in Postgres 13+.
                conn.execute(&*format!(
                    "SELECT pg_terminate_backend(pg_stat_activity.pid)
                    FROM pg_stat_activity
                    WHERE datname = '{}'
                      AND pid <> pg_backend_pid();",
                    db_name
                ))
                .await
                .expect("Failed to disconnect other sessions");

                conn.execute(&*format!("DROP DATABASE \"{}\";", db_name))
                    .await
                    .expect(&format!("Failed to drop temporary database: {}", db_name));
                println!("Dropped database: {db_name}");
                let _ = tx.send(());
            })
        });

        let _ = rx.recv();
        println!("ran test teardown");
    }
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();

    let body = r#"{"name": "Tom Malone", "email": "tom@malone.com"}"#;

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "tom@malone.com");
    assert_eq!(saved.name, "Tom Malone");
}

#[tokio::test]
async fn subscribe_returns_a_400_for_missing_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        (r#"{"name": "Tom Malone"}"#, "missing the mail"),
        (r#"{"email": "tom@malone.com"}"#, "missing the name"),
        ("", "missing email and name"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Conent-Type", "application/json")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request...");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port...");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to load config.");
    configuration.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&configuration.database).await;
    let server = run(listener, connection_pool.clone()).expect("Failed to bind address.");

    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
        db_name: configuration.database.database_name,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
