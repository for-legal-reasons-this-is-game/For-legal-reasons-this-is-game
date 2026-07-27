use axum::{
    extract::{Json, Path},
    http::{Response, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CreateUser {
    name: String,
}

//create user write it to database and respond if it sucseeds and create userid
pub async fn create_user(Json(payload): Json<Value>) -> Result<Response<String>, StatusCode> {
    let data: CreateUser = serde_json::from_value(payload).unwrap();
    println!("{}", data.name);
    todo!();
}

//create account with unique ID and write it to database
pub async fn create_account(Json(_payload): Json<Value>) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the user
pub async fn fetch_user(Path(_user_id): Path<Uuid>) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the account
pub async fn fetch_account(Path(_account_id): Path<Uuid>) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the accoiunts position
pub async fn fetch_positions(
    Path(_account_id): Path<Uuid>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the accoiunts position
pub async fn fetch_account_position_asset(
    Path(_account_id): Path<Uuid>,
    Path(_asset_id): Path<Uuid>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the accoiunts position
pub async fn delete_account(
    Path(_account_id): Path<Uuid>,
    Path(_asset_id): Path<Uuid>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn list_instruments() -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn fetch_instrument(Path(_symbol): Path<String>) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn create_instrument(
    Path(_symbol): Path<String>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn delete_instrument(
    Path(_symbol): Path<String>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}
