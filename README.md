# Banana

## Technical instructions

### Sea ORM

#### Installation
1. In Cargo.toml, add the following dependencies:
``` toml
sea-orm = { version = "1.0.0-rc.5", features = [ <DATABASE_DRIVER>, <ASYNC_RUNTIME>, "macros" ] }
```
2. See `config::database` for database configuration
3. Add migration crate `sea-orm-cli migrate init`
4. Configure migration crate `.toml` file and add the crate in the main crate
5. Add entity crate `cargo new entity`
6. Run the command to generate the entities and add the crate in dependencies of the main one

#### Commands
- `sea-orm-cli migrate generate <migration name>`: create a new migration
- `sea-orm-cli migrate up`: run migration
- `sea-orm-cli generate entity -o entity/src`: generate entities

### Actix
1. Add the dependency
2. Define the server and launch the main in async mode
```rust
pub async fn config(db_connection: &'static DatabaseConnection) -> std::io::Result<()> {
    info!("Starting Actix server");

    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(HandlerState {
                db_connection: db_connection
            }))
            .service(get_profile_by_id)
            .service(create_profile)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

config::actix::config(static_db).await;
```

### Logging
- See `config::logger`
- Use macros `log::info!`, `log::error!`, `log::warn!`, `log::debug!`, `log::trace!`

### OIDC
Pour supprimer le bout de code du front qui fait 
```
useEffect(() => {
    const code = queryParams.get('code');
    const sessionState = queryParams.get('session_state');
    const iss = queryParams.get('iss');
    if (isNotNullNorUndefined(code) && isNotNullNorUndefined(sessionState) && isNotNullNorUndefined(iss)) {
      authenticate(code, sessionState, iss)
      .then(() => {
        navigate('/');
      });
    }
  }, [queryParams]);
```
Il faut ajouter de la gestion de session ou alors trimballer un paramètre qui s'appelle genre 'next_url' qui est l'URL d'origine du front et rediriger vers cette URL tout à la fin. On peut alors paramétrer les redirect URLs pour que ça soit les URLs du client OIDC, donc le back, et non le front qui n'est pas le client.

Ou alors utiliser une session pour enregistrer l'URL d'origine et rediriger vers cette URL tout à la fin.