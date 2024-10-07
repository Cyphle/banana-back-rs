use crate::domain::profiles::CreateProfileCommand;

mod config;
mod repositories;
mod domain;

#[actix_web::main]
async fn main() {
    config::logger::config();

    log::info!("Starting the application");

    let db = config::database::connect().await.unwrap();
    let static_db = Box::leak(Box::new(db));

    // repositories::profiles::create(static_db, &CreateProfileCommand {
    //     username: "johndoe".to_string(),
    //     email: "johndoe".to_string(),
    //     first_name: "John".to_string(),
    //     last_name: "Doe".to_string(),
    // }).await.unwrap();


    log::info!("Application is now closed");
    // TODO faudra trouver un moyen de close la connexion.
    // db.close().unwrap()
}
