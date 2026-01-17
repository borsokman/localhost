#[derive(Debug, Clone, Copy)]
pub enum StatusCode {
    Ok,
    MovedPermanently,
    SeeOther,
    BadRequest,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    PayloadTooLarge,
    InternalServerError,
}

impl StatusCode {
    pub fn as_u16(self) -> u16 {
        match self {
            StatusCode::Ok => 200,
            StatusCode::MovedPermanently => 301,
            StatusCode::SeeOther => 303,
            StatusCode::BadRequest => 400,
            StatusCode::Forbidden => 403,
            StatusCode::NotFound => 404,
            StatusCode::MethodNotAllowed => 405,
            StatusCode::PayloadTooLarge => 413,
            StatusCode::InternalServerError => 500,
        }
    }
    pub fn reason(self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::MovedPermanently => "Moved Permanently",
            StatusCode::SeeOther => "See Other",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::PayloadTooLarge => "Payload Too Large",
            StatusCode::InternalServerError => "Internal Server Error",
        }
    }
}