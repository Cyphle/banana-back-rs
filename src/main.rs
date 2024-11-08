use actix_cors::Cors;
use actix_session::config::PersistentSession;
use actix_session::Session;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::{time, Key};
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use log::debug;
use log::error;
use log::info;
use openid::error::Error;
use openid::Bearer;
use openid::DiscoveredClient;
use openid::IdToken;
use openid::TokenIntrospection;
use openid::Userinfo;
use openid::{Client, Options, StandardClaims, Token};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

mod config;
mod domain;
mod dto;
mod http;
mod repositories;

#[derive(Debug, Deserialize)]
struct AuthRequest {
    code: Option<String>,
    session_state: Option<String>,
    iss: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    config::logger::config();

    log::info!("Starting the application");

    // let db = config::database::connect().await.unwrap();
    // let static_db = Box::leak(Box::new(db));

    // ACTIX
    // let _ = config::actix::config(static_db).await;

    // repositories::profiles::create(static_db, &CreateProfileCommand {
    //     username: "johndoe".to_string(),
    //     email: "johndoe".to_string(),
    //     first_name: "John".to_string(),
    //     last_name: "Doe".to_string(),
    // }).await.unwrap();

    // log::info!("Application is now closed");

    /* OPENID CONNECT */
    // Set up Keycloak OIDC parameters
    let issuer_url = reqwest::Url::parse("http://localhost:8181/realms/Banana");
    let client_id = "banana";
    let client_secret = "banana-secret";
    let redirect_uri = "http://localhost:9000/login";

    // Initialize OpenID Client with Keycloak discovery
    let client: Client<openid::Discovered, StandardClaims> = Client::discover(
        client_id.to_string(),
        client_secret.to_string(),
        Some(redirect_uri.to_string()),
        issuer_url.unwrap(),
    )
    .await
    .expect("Failed to discover OpenID configuration");

    // Wrap client in Arc and Mutex for sharing across Actix handlers
    let client = Arc::new(Mutex::new(client));
    // Generate a secure 32-byte key for cookie signing (use a random key in production)
    // let secret_key = Key::generate();
    let secret_key = Key::from(&[0; 64]);

    // Start Actix server
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
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(time::Duration::days(5)),
                    )
                    .cookie_secure(false)
                    .cookie_name("actix_cookie".to_string())
                    .build(),
            )
            .app_data(web::Data::new(client.clone()))
            .route("/set_session", web::get().to(set_session))
            .route("/get_session", web::get().to(get_session))
            .service(login)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await

    /* END OPENID CONNECT */

    // TODO faudra trouver un moyen de close la connexion. Mais là on peut pas move la static_db
}

////////// SESSION //////////

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CustomBearer {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub id_token: String,
    #[serde(flatten)]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

impl Display for CustomBearer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // match self.id_token.as_ref() {
        //     Some(id_token) => write!(f, "{} {}", self.access_token, id_token),
        //     None => write!(f, "{}", self.access_token),
        // }
        write!(f, "{} {}", self.access_token, self.id_token)
    }
}

// Route to set data in the session
async fn set_session(session: Session) -> impl Responder {
    let res = session.insert("user_id", 42);
    match res {
        Ok(_) => {
            info!("Session data set");
            HttpResponse::Ok().body("Session data set")
        },
        Err(e) => {
            error!("Error setting session data: {}", e);
            HttpResponse::Ok().body(format!("Error setting session data: {}", e))
        }
    }
}

fn save_token_in_session(session: &Session, token: &Bearer) {
    let saved = session.insert("token", token.clone());
    match saved {
        Ok(_) => info!("Token saved in session"),
        Err(e) => error!("Error saving token in session: {}", e),
    }
}

// Route to retrieve data from the session
async fn get_session(
    session: Session,
    client: web::Data<Arc<Mutex<Client<openid::Discovered, StandardClaims>>>>,
    _: web::Query<AuthRequest>,
) -> impl Responder {
    let user_id = session.get::<CustomBearer>("user_id");

    match user_id {
        Ok(user_id) => match user_id {
            Some(user_id) => {
                info!("User ID found in session: {}", user_id);
                HttpResponse::Ok().body(format!("Welcome back, user {}!", user_id))
            },
            None => {
                error!("No user ID found in session");
                HttpResponse::Ok().body("No user ID found in session")
            },
        },
        Err(e) => {
            error!("No session data found: {}", e);
            HttpResponse::Ok().body(format!("No session data found: {}", e))
        },
    }
}


////////// OIDC //////////
#[get("/login")]
async fn login(
    session: Session,
    client: web::Data<Arc<Mutex<Client<openid::Discovered, StandardClaims>>>>,
    query: web::Query<AuthRequest>,
) -> impl Responder {
    let client = client.lock().await;

    info!("Login with query: {:?}", query);

    match &query.code {
        Some(code) => {
            let authorization_code: &&String = &code;

            info!(
                "Requesting token with received authorization code: {}",
                authorization_code
            );

            match client.authenticate(authorization_code, None, None).await {
                Ok(token) => {
                    let access_token = token.bearer.access_token.clone();
                    let id_token = token.bearer.id_token.clone();
                    info!("ID token in authenticate: {}", id_token.clone().unwrap());

                    // save_token_in_session(&session, &token.bearer);
                    // Notes
                    /*
                        Bon ça marche pas de serializer tout le token en session. Il faut créer une autre struct qui save les token in memory et avec une clé un hash ou uuid et en session on save l'uuid
                     */
                    let bearer: Bearer = token.bearer.clone();
                    let another_bearer = CustomBearer {
                        access_token: bearer.access_token.clone(),
                        token_type: bearer.token_type,
                        scope: bearer.scope,
                        state: bearer.state,
                        refresh_token: bearer.refresh_token,
                        expires_in: bearer.expires_in,
                        id_token: id_token.clone().unwrap(),
                        extra: bearer.extra,
                    };
                    let saved = session.insert("user_id", another_bearer);
                    match saved {
                        Ok(_) => info!("Token saved in session"),
                        Err(e) => error!("Error saving token in session: {}", e),
                    }

                    HttpResponse::Ok().json(HashMap::from([
                        ("access_token", access_token),
                        ("id_token", id_token.unwrap_or_default()),
                    ]))
                }
                Err(err) => {
                    error!("Error exchanging code for token: {:?}", err);
                    HttpResponse::InternalServerError().body("Failed to authenticate")
                }
            }
        }
        None => {
            info!("No code provided. Starting authentication.");

            // Il faut définir un nonce et max age ici pour réutiliser à priori
            let auth_url = client.auth_url(&Options {
                scope: Some("openid email profile".into()),
                ..Default::default()
            });

            HttpResponse::Found()
                .append_header(("Location", auth_url.to_string()))
                .finish()
        }
    }
}

// GETTING INFO FROM ID TOKEN
/*
    How to use:
*/
fn get_info_from_id_token(id_token: &IdToken<StandardClaims>) -> Userinfo {
    id_token.payload().cloned().unwrap().userinfo
}

// VALIDATING/INTROSPECTING TOKEN
/*
    How to use:
    // Validating access token
    let token_from_bearer = introspect_token_from_bearer(&client, token.bearer.clone()).await;
    // Validating from token
    let introspection = introspect_token(&client, &token).await;

    match introspection {
        Ok(intro) => info!("Token introspection successful: {:?}", intro),
        Err(e) => error!("Token introspection failed: {:?}", e)
    }
    // End validating access token
*/
async fn introspect_token(
    client: &DiscoveredClient,
    token: &Token,
) -> Result<TokenIntrospection<StandardClaims>, Error> {
    client.request_token_introspection(&token).await
}

async fn introspect_token_from_bearer(
    client: &DiscoveredClient,
    bearer: Bearer,
) -> Result<TokenIntrospection<StandardClaims>, Error> {
    let token = Token::from(bearer);
    client.request_token_introspection(&token).await
}
