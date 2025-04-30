use serde::{Deserialize, Serialize};
use mongodb::bson::{oid::ObjectId, DateTime};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub sender_id: ObjectId,
    pub receiver_id: ObjectId,
    pub encrypted_key: String,
    pub encrypted_content: String,
    pub iv: String,
    pub unlock_date: DateTime,
    pub send_date: DateTime,
}
