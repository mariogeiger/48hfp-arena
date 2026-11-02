//! The wiki's HTTP side: editing, proposing and reviewing films, their
//! history, uploaded posters and the catalog as CSV. Refusals answer
//! `{"error": <code>}`, which the client words in its language.

use crate::catalog::Refusal;
use crate::config::Columns;
use crate::persistence::AppState;
use crate::throttle::Action;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, web};
use sha2::{Digest, Sha256};
use versus_shared::*;

/// Largest poster upload; the client sends a downscaled JPEG well below it.
pub const MAX_UPLOAD: usize = 1 << 20;
const MAX_TEXT: usize = 200;
const MAX_URL: usize = 2000;
const MAX_SYNOPSIS: usize = 2000;

fn refuse(status: StatusCode, code: &str) -> HttpResponse {
    HttpResponse::build(status).json(serde_json::json!({ "error": code }))
}

fn refusal(r: Refusal) -> HttpResponse {
    match r {
        Refusal::UnknownFilm => refuse(StatusCode::NOT_FOUND, "unknown_film"),
        Refusal::Unchanged => refuse(StatusCode::CONFLICT, "unchanged"),
        Refusal::NotPending => refuse(StatusCode::CONFLICT, "not_pending"),
        Refusal::OwnProposal => refuse(StatusCode::FORBIDDEN, "own_proposal"),
        Refusal::Unwritable => refuse(StatusCode::INTERNAL_SERVER_ERROR, "server"),
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Whether `user_id`, from the address of `req`, may write now.
fn admit(
    data: &AppState,
    req: &HttpRequest,
    user_id: &str,
    action: Action,
) -> Result<(), HttpResponse> {
    if !data.editable {
        return Err(refuse(StatusCode::FORBIDDEN, "read_only"));
    }
    if user_id.is_empty() || data.is_banned(user_id) {
        return Err(refuse(StatusCode::FORBIDDEN, "banned"));
    }
    let address = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();
    if !data.throttle.admit(action, &[user_id, &address]) {
        return Err(refuse(StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
    }
    Ok(())
}

/// Trimmed fields, or the code of the first rule they break: a title is
/// required, text is short and single-line, a synopsis may span lines, links
/// are http(s) or a poster uploaded here.
pub fn validated(f: FilmFields) -> Result<FilmFields, &'static str> {
    let text = |s: String| -> Result<String, &'static str> {
        let s = s.trim().to_string();
        if s.chars().any(char::is_control) || s.chars().count() > MAX_TEXT {
            Err("bad_text")
        } else {
            Ok(s)
        }
    };
    let synopsis = |s: String| -> Result<String, &'static str> {
        let s = s.trim().replace("\r\n", "\n");
        let stray = s.chars().any(|c| c.is_control() && c != '\n');
        if stray || s.chars().count() > MAX_SYNOPSIS {
            Err("bad_synopsis")
        } else {
            Ok(s)
        }
    };
    let link = |s: String, uploaded: bool| -> Result<String, &'static str> {
        let s = s.trim().to_string();
        let web = (s.starts_with("https://") || s.starts_with("http://"))
            && !s.chars().any(|c| c.is_whitespace() || c.is_control())
            && s.len() <= MAX_URL;
        let local = uploaded && s.strip_prefix("/uploads/").is_some_and(is_upload_name);
        if s.is_empty() || web || local {
            Ok(s)
        } else {
            Err("bad_url")
        }
    };
    let fields = FilmFields {
        title: text(f.title)?,
        team: text(f.team)?,
        city: text(f.city)?,
        poster_url: link(f.poster_url, true)?,
        video_url: link(f.video_url, false)?,
        synopsis: synopsis(f.synopsis)?,
    };
    if fields.title.is_empty() {
        return Err("title_required");
    }
    Ok(fields)
}

/// Append `change` by `user_id`, then publish and tell everyone.
fn commit(data: &AppState, user_id: &str, film: Option<usize>, change: Change) -> HttpResponse {
    let kind = change.film_change();
    let result = data
        .catalog
        .lock()
        .unwrap()
        .append(user_id, film, change, now());
    match result {
        Ok(film) => {
            data.publish();
            data.broadcast(user_id, LiveChange::Film { film, change: kind });
            HttpResponse::Ok().json(serde_json::json!({ "film": film }))
        }
        Err(r) => refusal(r),
    }
}

pub async fn edit(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<usize>,
    payload: web::Json<EditPayload>,
) -> HttpResponse {
    let payload = payload.into_inner();
    if let Err(response) = admit(&data, &req, &payload.user_id, Action::Edit) {
        return response;
    }
    match validated(payload.fields) {
        Ok(fields) => commit(
            &data,
            &payload.user_id,
            Some(*path),
            Change::Edit {
                fields,
                revert_of: None,
            },
        ),
        Err(code) => refuse(StatusCode::BAD_REQUEST, code),
    }
}

pub async fn revert(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<usize>,
    payload: web::Json<RevertPayload>,
) -> HttpResponse {
    if let Err(response) = admit(&data, &req, &payload.user_id, Action::Edit) {
        return response;
    }
    let film = *path;
    let fields = data.catalog.lock().unwrap().fields_at(film, payload.rev);
    let Some(fields) = fields else {
        return refuse(StatusCode::NOT_FOUND, "unknown_revision");
    };
    let change = Change::Edit {
        fields,
        revert_of: Some(payload.rev),
    };
    commit(&data, &payload.user_id, Some(film), change)
}

pub async fn propose(
    data: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<ProposePayload>,
) -> HttpResponse {
    let payload = payload.into_inner();
    if let Err(response) = admit(&data, &req, &payload.user_id, Action::Propose) {
        return response;
    }
    match validated(payload.fields) {
        Ok(fields) => commit(&data, &payload.user_id, None, Change::Propose { fields }),
        Err(code) => refuse(StatusCode::BAD_REQUEST, code),
    }
}

pub async fn review(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<usize>,
    payload: web::Json<ReviewPayload>,
) -> HttpResponse {
    if let Err(response) = admit(&data, &req, &payload.user_id, Action::Edit) {
        return response;
    }
    let change = Change::Review {
        approve: payload.approve,
    };
    commit(&data, &payload.user_id, Some(*path), change)
}

pub async fn history(
    data: web::Data<AppState>,
    path: web::Path<usize>,
    query: web::Query<UserQuery>,
) -> HttpResponse {
    let catalog = data.catalog.lock().unwrap();
    HttpResponse::Ok().json(serde_json::json!({
        "revisions": catalog.history(*path, &query.user_id),
        "original": catalog.fields_at(*path, 0),
    }))
}

pub async fn proposals(data: web::Data<AppState>, query: web::Query<UserQuery>) -> HttpResponse {
    HttpResponse::Ok().json(data.catalog.lock().unwrap().proposals(&query.user_id))
}

fn is_upload_name(name: &str) -> bool {
    name.strip_suffix(".jpg").is_some_and(|hash| {
        hash.len() == 32
            && hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Store a JPEG under the hash of its bytes, so the same image is one file
/// and an old revision's poster never changes under it.
pub async fn upload(
    data: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<UserQuery>,
    body: web::Bytes,
) -> HttpResponse {
    if let Err(response) = admit(&data, &req, &query.user_id, Action::Upload) {
        return response;
    }
    if body.len() > MAX_UPLOAD {
        return refuse(StatusCode::PAYLOAD_TOO_LARGE, "too_large");
    }
    if !body.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return refuse(StatusCode::UNSUPPORTED_MEDIA_TYPE, "not_jpeg");
    }
    let digest = Sha256::digest(&body);
    let hash: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("{hash}.jpg");
    let dir = std::path::Path::new(&data.uploads_dir);
    let path = dir.join(&name);
    if !path.exists() {
        let tmp = dir.join(format!(".{name}.tmp"));
        let written = std::fs::create_dir_all(dir)
            .and_then(|_| std::fs::write(&tmp, &body))
            .and_then(|_| std::fs::rename(&tmp, &path));
        if let Err(e) = written {
            log::error!("Cannot store upload {}: {e}", path.display());
            return refuse(StatusCode::INTERNAL_SERVER_ERROR, "server");
        }
    }
    HttpResponse::Ok().json(serde_json::json!({ "url": format!("/uploads/{name}") }))
}

pub async fn uploaded(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    if !is_upload_name(&path) {
        return HttpResponse::NotFound().finish();
    }
    match actix_files::NamedFile::open(std::path::Path::new(&data.uploads_dir).join(&*path)) {
        Ok(file) => {
            let mut response = file.into_response(&req);
            response.headers_mut().insert(
                actix_web::http::header::CACHE_CONTROL,
                actix_web::http::header::HeaderValue::from_static(
                    "public, max-age=31536000, immutable",
                ),
            );
            response
        }
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

/// The published catalog in the shape of `data.csv`, under its headers.
pub async fn films_csv(data: web::Data<AppState>, columns: web::Data<Columns>) -> HttpResponse {
    let quote = |s: &str| {
        if s.contains([',', '"', '\n']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let slots: [(Option<&String>, fn(&Film) -> &str); 6] = [
        (Some(&columns.label), |f| &f.title),
        (columns.sublabel.as_ref(), |f| &f.team),
        (columns.detail.as_ref(), |f| &f.city),
        (columns.image.as_ref(), |f| &f.poster_url),
        (columns.link.as_ref(), |f| &f.video_url),
        (columns.description.as_ref(), |f| &f.synopsis),
    ];
    let slots: Vec<(&String, fn(&Film) -> &str)> = slots
        .into_iter()
        .filter_map(|(header, get)| Some((header?, get)))
        .collect();
    let line = |cells: Vec<String>| cells.join(",") + "\n";
    let mut csv = line(slots.iter().map(|(header, _)| quote(header)).collect());
    let films = data.films();
    let mut films: Vec<&Film> = films.values().collect();
    films.sort_by_key(|f| f.id);
    for film in films {
        csv.push_str(&line(
            slots.iter().map(|(_, get)| quote(get(film))).collect(),
        ));
    }
    HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .body(csv)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(title: &str, poster: &str, video: &str) -> FilmFields {
        FilmFields {
            title: title.into(),
            poster_url: poster.into(),
            video_url: video.into(),
            ..Default::default()
        }
    }

    #[test]
    fn links_are_web_or_uploaded_posters() {
        let upload = format!("/uploads/{}.jpg", "0".repeat(32));
        assert!(validated(fields(" T ", &upload, "https://x.ch/v.mov")).is_ok());
        assert_eq!(
            validated(fields("T", "", "javascript:alert(1)")),
            Err("bad_url")
        );
        assert_eq!(validated(fields("T", "", &upload)), Err("bad_url"));
        assert_eq!(
            validated(fields("T", "/uploads/../db.json", "")),
            Err("bad_url")
        );
        assert_eq!(validated(fields("  ", "", "")), Err("title_required"));
        assert_eq!(validated(fields("a\nb", "", "")), Err("bad_text"));
        let mut long = fields("T", "", "");
        long.synopsis = " Une ligne.\r\nUne autre. ".into();
        assert_eq!(validated(long).unwrap().synopsis, "Une ligne.\nUne autre.");
    }
}
