use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse,
};
use actix_web::body::BoxBody;
use jsonwebtoken::{decode, Validation, DecodingKey};
use crate::handlers::Claims;
use bson::doc;
use actix_web::HttpMessage;

pub async fn jwt_middleware(
    req: ServiceRequest,
    next: actix_web::middleware::Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let secret_key = req
        .app_data::<actix_web::web::Data<String>>() // Récupérer la clé secrète depuis les données d'application
        .map(|data| data.get_ref())
        .unwrap();

    if let Some(auth_header) = req.headers().get(actix_web::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = auth_str.trim_start_matches("Bearer ");

                // Validation du token avec l'expiration activée
                let mut validation = Validation::default();
                validation.validate_exp = true;

                if let Ok(decoded) = decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(secret_key.as_bytes()),
                    &validation,
                ) {
                    // Insère l'utilisateur dans les extensions de la requête
                    req.extensions_mut()
                        .insert(decoded.claims.sub.clone());

                    // Passe au service suivant
                    return next.call(req).await;
                }
            }
        }
    }

    // Si le token est invalide ou expiré, renvoie une réponse 401 Unauthorized
    let response = req.into_response(
        HttpResponse::Unauthorized()
            .json(doc! { "error": "Token invalide ou expiré." })
            .map_into_boxed_body(),
    );
    Ok(response)
}
