use actix_web::{web, HttpResponse, Responder};
use mongodb::{bson::doc, bson::Document, Database};
use ring::{rand, digest, pbkdf2};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use mongodb::bson::{DateTime};
use crate::message_services::MessageService;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _; // Pour activer les méthodes encode/decode
use futures::stream::StreamExt;
use actix_web::HttpMessage;

/// Partie concernant la gestion des messages



#[derive(Debug, Deserialize)]
pub struct SendMessageInput {
    pub receiver_username: String,
    pub content: String,
    pub unlock_date: String, // Format: ISO 8601
}
pub async fn send_message(
    db: web::Data<Database>,
    input: web::Json<SendMessageInput>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let collection_users = db.collection::<Document>("users");
    let collection_messages = db.collection::<Document>("messages");

    // Récupérer l'identifiant de l'expéditeur à partir des extensions
    let sender_username = req.extensions().get::<String>().cloned();
    if sender_username.is_none() {
        return HttpResponse::Unauthorized().json(doc! { "error": "Utilisateur non authentifié." });
    }
    let sender_username = sender_username.unwrap();

    // Rechercher l'identifiant de l'utilisateur expéditeur
    let sender = collection_users
        .find_one(doc! { "username": &sender_username })
        .await;

    let sender_id = match sender {
        Ok(Some(user)) => user.get_object_id("_id").unwrap(),
        _ => return HttpResponse::InternalServerError().json(doc! { "error": "Erreur interne." }),
    };

    // Rechercher le destinataire
    let receiver = collection_users
        .find_one(doc! { "username": &input.receiver_username })
        .await;

    let receiver_id = match receiver {
        Ok(Some(user)) => user.get_object_id("_id").unwrap(),
        _ => return HttpResponse::NotFound().json(doc! { "error": "Destinataire introuvable." }),
    };

    // Chiffrement
    let aes_key = MessageService::generate_aes_key();
    let iv = MessageService::generate_iv();
    let (encrypted_content, encoded_iv) = MessageService::encrypt_message(&input.content, &aes_key, iv);

    // Clé AES chiffrée (implémentation ECC requise ici)
    let encrypted_key = STANDARD.encode(aes_key); // Placeholder

    // Ajouter le message dans la base de données
    let message = doc! {
        "sender_id": sender_id,
        "receiver_id": receiver_id,
        "encrypted_key": encrypted_key,
        "encrypted_content": encrypted_content,
        "iv": encoded_iv,
        "unlock_date": DateTime::parse_rfc3339_str(&input.unlock_date).unwrap(),
        "send_date": DateTime::now(),
    };

    match collection_messages.insert_one(message).await {
        Ok(_) => HttpResponse::Created().json(doc! { "message": "Message envoyé avec succès." }),
        Err(err) => HttpResponse::InternalServerError().json(doc! { "error": format!("Erreur : {}", err) }),
    }
}



pub async fn receive_messages(
    db: web::Data<Database>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let collection = db.collection::<Document>("messages");

    // Récupérer l'identifiant de l'utilisateur connecté depuis les extensions
    let receiver_username = req.extensions().get::<String>().cloned();
    if receiver_username.is_none() {
        return HttpResponse::Unauthorized().json(doc! { "error": "Utilisateur non authentifié." });
    }
    let receiver_username = receiver_username.unwrap();

    // Rechercher l'utilisateur dans la base de données pour obtenir son ObjectId
    let collection_users = db.collection::<Document>("users");
    let receiver = collection_users
        .find_one(doc! { "username": &receiver_username })
        .await;

    let receiver_id = match receiver {
        Ok(Some(user)) => user.get_object_id("_id").unwrap(),
        _ => return HttpResponse::InternalServerError().json(doc! { "error": "Erreur interne." }),
    };

    // Filtrer les messages par receiver_id
    let filter = doc! { "receiver_id": receiver_id };
    let mut cursor = collection.find(filter).await.unwrap();

    let mut response = vec![];

    while let Some(result) = cursor.next().await {
        if let Ok(message) = result {
            let unlock_date: DateTime = *message.get_datetime("unlock_date").unwrap();

            // Déchiffrer le message si la date est atteinte
            let content = if unlock_date <= DateTime::now() {
                let encrypted_key = STANDARD
                    .decode(message.get_str("encrypted_key").unwrap())
                    .unwrap();
                let encrypted_content = message.get_str("encrypted_content").unwrap();
                let iv: [u8; 12] = STANDARD
                    .decode(message.get_str("iv").unwrap())
                    .unwrap()
                    .try_into()
                    .expect("IV de taille incorrecte");
            
                match MessageService::decrypt_message(encrypted_content, &encrypted_key, iv) {
                    Ok(decrypted_message) => decrypted_message,
                    Err(err) => format!("Erreur de déchiffrement : {}", err),
                }
            } else {
                "Message non déverrouillé.".to_string()
            };

            response.push(doc! {
                "sender_id": message.get_object_id("sender_id").unwrap(),
                "content": content,
                "unlock_date": unlock_date.to_string(),
            });
        }
    }

    HttpResponse::Ok().json(response)
}

/// Partie concernant la gestion des utilisateurs

const ITERATIONS: u32 = 100_000; // Nombre d'itérations pour PBKDF2
const SALT_LEN: usize = 16;      // Longueur du sel
const KEY_LEN: usize = digest::SHA256_OUTPUT_LEN;

#[derive(Debug, Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Nom d'utilisateur
    pub exp: usize,  // Expiration du token (timestamp Unix)
}

pub async fn register_user(
    db: web::Data<Database>,
    input: web::Json<RegisterInput>,
) -> impl Responder {
    let collection = db.collection::<Document>("users");

    // Vérifier si le nom d'utilisateur existe déjà
    let filter = doc! { "username": &input.username };
    if let Ok(Some(_)) = collection.find_one(filter).await {
        return HttpResponse::Conflict().json(doc! { "error": "Nom d'utilisateur déjà utilisé." });
    }

    // Générer un sel sécurisé
    let rng = rand::SystemRandom::new();
    let mut salt = [0u8; SALT_LEN];
    rng.fill(&mut salt).unwrap();

    // Hacher le mot de passe
    let mut hashed_password = [0u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(ITERATIONS).unwrap(),
        &salt,
        input.password.as_bytes(),
        &mut hashed_password,
    );

    let hashed_password_hex = hex::encode(hashed_password);
    let salt_hex = hex::encode(salt);

    let new_user = doc! {
        "username": &input.username,
        "hashed_password": hashed_password_hex,
        "salt": salt_hex,
    };

    match collection.insert_one(new_user).await {
        Ok(_) => HttpResponse::Created().json(doc! { "message": "Utilisateur enregistré avec succès." }),
        Err(err) => HttpResponse::InternalServerError().json(doc! { "error": format!("Erreur : {}", err) }),
    }
}

pub async fn login_user(
    db: web::Data<Database>,
    secret_key: web::Data<String>, // Récupérer la clé secrète depuis les données d'application
    input: web::Json<LoginInput>,
) -> impl Responder {
    let collection = db.collection::<Document>("users");

    let filter = doc! { "username": &input.username };
    match collection.find_one(filter).await {
        Ok(Some(user)) => {
            let hashed_password: String = user.get_str("hashed_password").unwrap().to_string();
            let salt_hex: String = user.get_str("salt").unwrap().to_string();

            let salt = hex::decode(salt_hex).unwrap();
            let stored_hash = hex::decode(hashed_password).unwrap();

            if pbkdf2::verify(
                pbkdf2::PBKDF2_HMAC_SHA256,
                NonZeroU32::new(ITERATIONS).unwrap(),
                &salt,
                input.password.as_bytes(),
                &stored_hash,
            )
            .is_ok()
            {
                let expiration = Utc::now()
                    .checked_add_signed(Duration::hours(24))
                    .expect("Erreur lors de la génération de la date d'expiration")
                    .timestamp() as usize;

                let claims = Claims {
                    sub: input.username.clone(),
                    exp: expiration,
                };

                let token = encode(
                    &Header::default(),
                    &claims,
                    &EncodingKey::from_secret(secret_key.as_ref().as_bytes()),
                )
                .unwrap();

                return HttpResponse::Ok().json(doc! { "token": token });
            } else {
                HttpResponse::Unauthorized().json(doc! { "error": "Mot de passe incorrect." })
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(doc! { "error": "Utilisateur non trouvé." }),
        Err(err) => HttpResponse::InternalServerError().json(doc! { "error": format!("Erreur : {}", err) }),
    }
}


