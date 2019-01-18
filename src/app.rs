use crate::actix::prelude::*;
use actix_web::{self, fs, middleware};
use actix_web::{App, http::Method, HttpRequest, fs::NamedFile};
use crate::models::DbExecutor;
use std::path::PathBuf;
use std::path::Path;
use std::sync::Arc;

use crate::api;
use crate::tokens::{TokenParser};
use crate::jobs::{JobQueue};
use actix_web::dev::FromParam;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // "build", "build/N"
    pub scope: Vec<String>, // "build", "upload" "publish"
    pub prefix: Option<Vec<String>>,
    pub name: String, // for debug/logs only
    pub exp: i64,
}

pub struct Config {
    pub repo_path: PathBuf,
    pub build_repo_base_path: PathBuf,
    pub base_url: String,
    pub collection_id: Option<String>,
    pub gpg_homedir: Option<String>,
    pub build_gpg_key : Option<String>,
    pub build_gpg_key_content : Option<String>,
    pub main_gpg_key: Option<String>,
    pub main_gpg_key_content: Option<String>,
    pub secret: Vec<u8>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub config: Arc<Config>,
    pub job_queue: Addr<JobQueue>,
}

fn handle_build_repo(req: &HttpRequest<AppState>) -> actix_web::Result<NamedFile> {
    let tail: String = req.match_info().query("tail")?;
    let id: String = req.match_info().query("id")?;
    let state = req.state();
    // Strip out any "../.." or other unsafe things
    let relpath = PathBuf::from_param(tail.trim_left_matches('/'))?;
    // The id won't have slashes, but it could have ".." or some other unsafe thing
    let safe_id = PathBuf::from_param(&id)?;
    let path = Path::new(&state.config.build_repo_base_path).join(&safe_id).join(&relpath);
    NamedFile::open(path).or_else(|_e| {
        let fallback_path = Path::new(&state.config.repo_path).join(relpath);
        Ok(NamedFile::open(fallback_path)?)
    })
}

pub fn create_app(
    db: Addr<DbExecutor>,
    config: &Arc<Config>,
    job_queue: Addr<JobQueue>,
) -> App<AppState> {
    let state = AppState {
        db: db.clone(),
        job_queue: job_queue.clone(),
        config: config.clone(),
    };

    let repo_static_files = fs::StaticFiles::new(&state.config.repo_path)
        .expect("failed constructing repo handler");

    App::with_state(state)
        .middleware(middleware::Logger::default())
        .scope("/api/v1", |scope| {
            scope
                .middleware(TokenParser::new(&config.secret))
                .resource("/token_subset", |r| r.method(Method::POST).with(api::token_subset))
                .resource("/job/{id}", |r| { r.name("show_job"); r.method(Method::POST).with(api::get_job)})
                .resource("/build", |r| { r.method(Method::POST).with(api::create_build);
                                          r.method(Method::GET).with(api::builds) })
                .resource("/build/{id}", |r| { r.name("show_build"); r.method(Method::GET).with(api::get_build) })
                .resource("/build/{id}/build_ref", |r| r.method(Method::POST).with(api::create_build_ref))
                .resource("/build/{id}/build_ref/{ref_id}", |r| { r.name("show_build_ref"); r.method(Method::GET).with(api::get_build_ref) })
                .resource("/build/{id}/missing_objects", |r| r.method(Method::GET).with(api::missing_objects))
                .resource("/build/{id}/upload", |r| r.method(Method::POST).with(api::upload))
                .resource("/build/{id}/commit", |r| { r.name("show_commit_job");
                                                      r.method(Method::POST).with(api::commit);
                                                      r.method(Method::GET).with(api::get_commit_job) })
                .resource("/build/{id}/publish", |r| { r.name("show_publish_job");
                                                      r.method(Method::POST).with(api::publish);
                                                       r.method(Method::GET).with(api::get_publish_job) })
                .resource("/build/{id}/purge", |r| { r.method(Method::POST).with(api::purge) })
        })
        .scope("/build-repo/{id}", |scope| {
            scope.handler("/", |req: &HttpRequest<AppState>| handle_build_repo(req))
        })
        .handler("/repo", repo_static_files)
}
