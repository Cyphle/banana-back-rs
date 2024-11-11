use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    config::{
        database::connect,
        local::{
            database_config::get_database_config, oidc_config::get_oidc_config,
            session_config::get_session_config,
        },
    },
    http::handlers::profile::{create_profile, get_profile_by_id},
    security::oidc::get_client,
};
use actix_cors::Cors;
use actix_session::{config::PersistentSession, storage::RedisSessionStore, SessionMiddleware};
use actix_web::{
    cookie::{time, Key},
    web, App, HttpServer,
};
use log::info;
use openid::{Bearer, Client, StandardClaims};
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

pub struct AppState {
    pub db_connection: &'static DatabaseConnection,
    pub client: Arc<Mutex<Client<openid::Discovered, StandardClaims>>>,
    pub store: Mutex<HashMap<String, Bearer>>,
}

pub struct SessionConfig {
    pub store_addr: String,
    pub cookie_name: String,
}

pub async fn config() -> std::io::Result<()> {
    info!("Starting Actix server");

    // Databse
    let db = connect(&get_database_config()).await.unwrap();
    let static_db = Box::leak(Box::new(db));

    // OIDC
    let client = Arc::new(Mutex::new(get_client(&get_oidc_config()).await));

    // Session
    let session_config = get_session_config();
    let secret_key = Key::from(&[0; 64]);
    let redis_store = RedisSessionStore::new(session_config.store_addr)
        .await
        .unwrap();

    let state = web::Data::new(AppState {
        db_connection: static_db,
        client: client.clone(),
        store: Mutex::new(HashMap::new()),
    });

    // Actix
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:9000") // Change to your frontend URL
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![actix_web::http::header::CONTENT_TYPE])
                    .supports_credentials() // Optional, if credentials are used
                    .max_age(3600),
            )
            .wrap(
                SessionMiddleware::builder(redis_store.clone(), secret_key.clone())
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(time::Duration::days(5)),
                    )
                    .cookie_secure(false)
                    .cookie_name(session_config.cookie_name.clone())
                    .build(),
            )
            .app_data(state.clone())
            .service(get_profile_by_id)
            .service(create_profile)
        // .route("/get_session", web::get().to(get_session))
        // .route("/set", web::get().to(set_key))
        // .route("/get", web::get().to(get_key))
        // .service(login)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
