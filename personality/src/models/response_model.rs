use rocket::Responder;

#[derive(Responder, Debug)]
pub enum ApiResponse {
    #[response(status = 401)]
    Unauthorized(String),
}
