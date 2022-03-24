use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;

    let body = r#"{"name": "Tom Malone", "email": "tom@malone.com"}"#;

    let response = app.post_subscriptions(body.into()).await;

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

    let test_cases = vec![
        (r#"{"name": "Tom Malone"}"#, "missing the mail"),
        (r#"{"email": "tom@malone.com"}"#, "missing the name"),
        ("", "missing email and name"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;

    let test_cases = vec![
        (
            r#"{"name": "Tom {} esle", "email": "tom@malone"}"#,
            "invalid chars { and } in name",
        ),
        (
            r#"{"name": "   ", "email": "tom1@malone"}"#,
            "just spaces in name",
        ),
        (
            r#"{"name": "Tom esleasdlkfjasfjassdieo20349gfjlsjagoijaiojo0a909alskdjflasljaslöaslöfjasdlfjaslfdjalsdfjlöasdflasldflasdlfasdlfdlasdflasdlfasdflöasdlfölasjdflöasdlöfkalöskdflökasdflökalöskflökasdflköasdflökaslökfjlköasdflökjasldöfkjalöksdflökkasdflöjasölfdjasdölfjasölfkdjasölfdjasdölfjasöldfkjölasjdfölasdjfölkajsdfölaksjdfölaksjdfölaksjdfaölskdjfölaksdföalskdfölaksdjfaölskidjf", "email": "tom2@malone"}"#,
            "name is too long",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}
