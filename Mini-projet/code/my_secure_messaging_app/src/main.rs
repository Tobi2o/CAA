use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use mongodb::{Client, options::ClientOptions};
use actix_web::middleware::from_fn;
use bson::oid::ObjectId;

mod handlers;
mod middleware;
mod message_services;
use crate::middleware::jwt_middleware;
use env_logger;


use dotenv::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Charger les variables d'environnement
    dotenv().ok();
    env_logger::init();

    // Lire la clé secrète depuis les variables d'environnement
    let secret_key = env::var("SECRET_KEY").expect("Clé secrète manquante dans les variables d'environnement.");

    // Configuration de MongoDB
    let client_options = ClientOptions::parse("mongodb://localhost:27017").await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("secure_messaging_app");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(secret_key.clone())) // Clé secrète
            .app_data(web::Data::new(ObjectId::new()))    // Exemple de sender_id temporaire
            .route("/", web::get().to(welcome))
            .route("/register", web::post().to(handlers::register_user))
            .route("/login", web::post().to(handlers::login_user))
            .service(
                web::scope("/protected")
                    .wrap(from_fn(jwt_middleware))
                    .route("", web::get().to(protected_route)),
            )
            .service(
                web::scope("/messages")
                    .wrap(from_fn(jwt_middleware)) // Appliquer le middleware ici
                    .route("/send", web::post().to(handlers::send_message))
                    .route("/receive", web::get().to(handlers::receive_messages)),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}



// Gestionnaire pour la route "/"
async fn welcome() -> impl Responder {
    HttpResponse::Ok().body("Bienvenue sur l'application de messagerie sécurisée!")
}

// Gestionnaire pour la route protégée
async fn protected_route() -> impl Responder {
    HttpResponse::Ok().body("Bienvenue sur une route protégée!")
}
