use axum::{extract::Path, response::IntoResponse};

pub async fn route_call(Path(route_info): Path<[String;2]>) -> impl IntoResponse{
   todo!() 
}
