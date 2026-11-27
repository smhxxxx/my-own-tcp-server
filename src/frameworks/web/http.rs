use std::sync::{Mutex, MutexGuard, OnceLock};
use crate::applications::async_web::http::{HttpFunction, Method};
use crate::applications::async_web::protocol::{Http, Phase};

static HTTP_SETUP:OnceLock<Mutex<Option<Http>>> = OnceLock::new();

fn http_as_guard() -> MutexGuard<'static, Option<Http>>
{
    HTTP_SETUP.get_or_init(|| {
            Mutex::new(Some(Http::new()))
        })
        .lock().unwrap()
}

pub fn route(method:Method, endpoint:&str, function: HttpFunction)
{
    http_as_guard()
        .as_mut().unwrap()
        .set_handler(method, endpoint, function);
}

pub fn filter(phase:Phase, pattern:&str, function: HttpFunction)
{
    match phase {
        Phase::PreHandle => http_as_guard()
            .as_mut().unwrap()
            .set_pre_handler(pattern, function),
        Phase::PostHandle => http_as_guard()
            .as_mut().unwrap()
            .set_post_handler(pattern, function),
    };
}

pub fn extract_http_setup() -> Http {
    HTTP_SETUP.get().unwrap()
        .lock().unwrap()
        .take().unwrap()
}

#[macro_export]
macro_rules! handler {
    // 인라인 블록 => 핸들러
    ($req:ident, $res:ident, $body:block) => {{
        fn __wrapper<'a>(
            $req: &'a mut $crate::applications::async_web::http::HttpRequest,
            $res: &'a mut $crate::applications::async_web::http::HttpResponse,
        ) -> $crate::applications::async_web::http::AsyncResult<'a> {
            Box::pin(async move $body)
        }
        __wrapper as $crate::applications::async_web::http::HttpFunction
    }};
}
