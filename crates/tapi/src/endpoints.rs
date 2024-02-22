use futures_util::StreamExt;
use itertools::Itertools;

use crate::{targets::ts, transitive_closure, DynTapi, Tapi};

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}
impl Method {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RequestStructureBody {
    Query(DynTapi),
    Json(DynTapi),
    PlainText,
}
#[derive(Debug)]
pub struct RequestStructure {
    pub path: Option<DynTapi>,
    pub method: Method,
    pub body: Option<RequestStructureBody>,
}

impl RequestStructure {
    pub fn new(method: Method) -> Self {
        Self {
            path: None,
            method,
            body: None,
        }
    }
    pub fn merge_with(&mut self, req: RequestTapi) {
        match req {
            RequestTapi::Path(ty) => {
                self.path = Some(ty);
            }
            RequestTapi::Query(ty) => {
                self.body = Some(RequestStructureBody::Query(ty));
            }
            RequestTapi::Json(ty) => {
                self.body = Some(RequestStructureBody::Json(ty));
            }
            RequestTapi::None => {}
        }
    }
}
pub trait Endpoint<AppState> {
    fn path(&self) -> &'static str;
    fn method(&self) -> Method;
    fn bind_to(&self, router: axum::Router<AppState>) -> axum::Router<AppState>;
    fn body(&self) -> RequestStructure;
    fn res(&self) -> ResponseTapi;
    fn tys(&self) -> Vec<DynTapi> {
        let mut tys = Vec::new();
        if let Some(path) = self.body().path {
            tys.push(path);
        }
        if let Some(body) = self.body().body {
            match body {
                RequestStructureBody::Query(ty) => {
                    tys.push(ty);
                }
                RequestStructureBody::Json(ty) => {
                    tys.push(ty);
                }
                RequestStructureBody::PlainText => {}
            }
        }
        tys.push(self.res().ty());
        tys
    }
    /// Generate a TypeScript client for this endpoint.
    ///
    /// The generated client will look something like this:
    /// ```ignore
    /// export const api = {
    ///     index: request<{}, string>("none", "GET", "/", "text"),
    ///     api: request<Person, string>("json", "GET", "/api", "json"),
    ///     api2AB: request<{}, string>("none", "GET", "/api2/:a/:b", "text"),
    ///     wow: sse<Msg>("/wow", "json"),
    ///     cool: request<Record<string, string>, Msg>("json", "GET", "/cool", "json"),
    /// };
    /// ```
    fn ts_client(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        match (self.body(), self.res()) {
            (
                RequestStructure {
                    body: None, path, ..
                },
                ResponseTapi::Sse(ty),
            ) => {
                let mut params = Vec::new();
                let final_path = self
                    .path()
                    .split('/')
                    .filter(|p| !p.is_empty())
                    .map(|p| {
                        if let Some(name) = p.strip_prefix(':') {
                            params.push(name);
                            format!("/${{{name}}}")
                        } else {
                            format!("/{p}")
                        }
                    })
                    .join("");
                let final_path = format!("`{final_path}`");
                if let Some(path_param) = path {
                    write!(
                        s,
                        "sse<[{}], {}>(({}) => {final_path}, \"json\")",
                        ts::full_ty_name(path_param),
                        ts::full_ty_name(ty),
                        params.iter().format(", "),
                    )
                    .unwrap();
                } else {
                    // TODO: handle non-json responses
                    write!(
                        s,
                        "sse<[{}], {}>(({}) => {final_path}, \"json\")",
                        "",
                        ts::full_ty_name(ty),
                        params.iter().format(", "),
                    )
                    .unwrap();
                }
            }
            (RequestStructure { body, .. }, res) => {
                write!(
                    s,
                    "request<{}, {}>({:?}, {:?}, {:?}, {:?})",
                    match body {
                        Some(RequestStructureBody::Query(ty)) => ts::full_ty_name(ty),
                        Some(RequestStructureBody::Json(ty)) => ts::full_ty_name(ty),
                        // TODO: is this right?
                        Some(RequestStructureBody::PlainText) =>
                            "Record<string, never>".to_string(),
                        None => "Record<string, never>".to_string(),
                    },
                    ts::full_ty_name(res.ty()),
                    match body {
                        Some(RequestStructureBody::Query(_)) => "query",
                        Some(RequestStructureBody::Json(_)) => "json",
                        Some(RequestStructureBody::PlainText) => "none",
                        None => "none",
                    },
                    self.method().as_str(),
                    self.path(),
                    match res {
                        ResponseTapi::PlainText => "text",
                        ResponseTapi::Bytes => "bytes",
                        ResponseTapi::Json(_) => "json",
                        ResponseTapi::Html => "html",
                        ResponseTapi::Sse(_) => "sse",
                        ResponseTapi::None => "none",
                    }
                )
                .unwrap();
            }
        }
        s
    }
}
impl<'a, AppState, T> Endpoint<AppState> for &'a T
where
    T: Endpoint<AppState>,
{
    fn path(&self) -> &'static str {
        (*self).path()
    }
    fn method(&self) -> Method {
        (*self).method()
    }
    fn bind_to(&self, router: axum::Router<AppState>) -> axum::Router<AppState> {
        (*self).bind_to(router)
    }
    fn body(&self) -> RequestStructure {
        (*self).body()
    }
    fn res(&self) -> ResponseTapi {
        (*self).res()
    }
}

pub struct Endpoints<'a, AppState> {
    endpoints: Vec<&'a dyn Endpoint<AppState>>,
    extra_tys: Vec<DynTapi>,
}
impl<'a, AppState> Endpoints<'a, AppState> {
    pub fn new(endpoints: impl IntoIterator<Item = &'a dyn Endpoint<AppState>>) -> Self {
        Self {
            endpoints: endpoints.into_iter().collect(),
            extra_tys: Vec::new(),
        }
    }
    pub fn with_ty<T: Tapi + 'static>(mut self) -> Self {
        self.extra_tys.push(T::boxed());
        self
    }
    pub fn tys(&self) -> Vec<DynTapi> {
        let mut tys = self.extra_tys.clone();
        for endpoint in &self.endpoints {
            tys.extend(endpoint.tys());
        }
        tys.sort_by_key(|t| t.id());
        tys.dedup_by_key(|t| t.id());
        transitive_closure(tys)
    }
    pub fn ts_client(&self) -> String {
        let mut s = ts::builder().types(self.tys());

        s.push_str("export const api = {\n");
        for endpoint in &self.endpoints {
            let name = heck::AsLowerCamelCase(endpoint.path()).to_string();
            let name = if name.is_empty() { "index" } else { &name };
            s.push_str(&format!("    {name}: {},\n", endpoint.ts_client()));
        }
        s.push_str("};\n");
        s
    }
}
impl<'a, AppState> IntoIterator for Endpoints<'a, AppState> {
    type Item = &'a dyn Endpoint<AppState>;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.endpoints.into_iter()
    }
}
impl<'s, 'a, AppState> IntoIterator for &'s Endpoints<'a, AppState> {
    type Item = &'a dyn Endpoint<AppState>;
    type IntoIter = std::iter::Copied<std::slice::Iter<'s, &'a dyn Endpoint<AppState>>>;
    fn into_iter(self) -> Self::IntoIter {
        self.endpoints.iter().copied()
    }
}

pub trait RouterExt<AppState: 'static> {
    fn tapi<E: Endpoint<AppState> + ?Sized>(self, endpoint: &E) -> Self;
    fn tapis<'a>(self, endpoints: impl IntoIterator<Item = &'a dyn Endpoint<AppState>>) -> Self
    where
        Self: Sized,
    {
        endpoints.into_iter().fold(self, Self::tapi)
    }
}

impl<AppState: 'static> RouterExt<AppState> for axum::Router<AppState> {
    fn tapi<E: Endpoint<AppState> + ?Sized>(self, endpoint: &E) -> Self {
        E::bind_to(endpoint, self)
    }
}

pub struct Sse<T, E = axum::BoxError>(futures_util::stream::BoxStream<'static, Result<T, E>>);
impl<T, E> Sse<T, E> {
    pub fn new<S>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = Result<T, E>> + Send + 'static,
    {
        Self(stream.boxed())
    }
}
impl<T> axum::response::IntoResponse for Sse<T>
where
    T: serde::Serialize + 'static,
{
    fn into_response(self) -> axum::response::Response {
        let stream = self
            .0
            .map(|s| -> Result<axum::response::sse::Event, axum::BoxError> {
                let s = serde_json::to_string(&s?)?;
                Ok(axum::response::sse::Event::default().data(s))
            });
        axum::response::sse::Sse::new(stream).into_response()
    }
}

#[derive(Debug)]
pub enum RequestTapi {
    Path(DynTapi),
    Query(DynTapi),
    Json(DynTapi),
    None,
}
pub trait RequestTapiExtractor {
    fn extract_request() -> RequestTapi;
}
impl RequestTapiExtractor for () {
    fn extract_request() -> RequestTapi {
        RequestTapi::None
    }
}
impl<T: Tapi + 'static> RequestTapiExtractor for axum::extract::Path<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::Path(<T as Tapi>::boxed())
    }
}
impl<T: Tapi + 'static> RequestTapiExtractor for axum::extract::Query<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::Query(<T as Tapi>::boxed())
    }
}
impl<T: Tapi + 'static> RequestTapiExtractor for axum::Json<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::Json(<T as Tapi>::boxed())
    }
}
impl<T> RequestTapiExtractor for axum::extract::State<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::None
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResponseTapi {
    // `text/plain; charset=utf-8`
    PlainText,
    // `application/octet-stream`
    Bytes,
    // `application/json`
    Json(DynTapi),
    // `text/html`
    Html,
    // `text/event-stream`
    Sse(DynTapi),
    None,
}
pub trait ResponseTapiExtractor {
    fn extract_response() -> ResponseTapi;
}
impl ResponseTapiExtractor for () {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::None
    }
}
impl ResponseTapiExtractor for String {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::PlainText
    }
}
impl ResponseTapiExtractor for Vec<u8> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Bytes
    }
}
impl<T: Tapi + 'static> ResponseTapiExtractor for axum::Json<T> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Json(<T as Tapi>::boxed())
    }
}
impl<T: Tapi + 'static> ResponseTapiExtractor for axum::response::Html<T> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Html
    }
}
impl<T: Tapi + 'static> ResponseTapiExtractor for Sse<T> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Sse(<T as Tapi>::boxed())
    }
}

impl RequestTapi {
    pub fn ty(self) -> DynTapi {
        match self {
            Self::Path(ty) => ty,
            Self::Query(ty) => ty,
            Self::Json(ty) => ty,
            Self::None => <() as Tapi>::boxed(),
        }
    }
}
impl ResponseTapi {
    pub fn ty(self) -> DynTapi {
        match self {
            Self::PlainText => <String as Tapi>::boxed(),
            Self::Bytes => <Vec<u8> as Tapi>::boxed(),
            Self::Json(ty) => ty,
            Self::Html => <String as Tapi>::boxed(),
            Self::Sse(ty) => ty,
            Self::None => <() as Tapi>::boxed(),
        }
    }
}
