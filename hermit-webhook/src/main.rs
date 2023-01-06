use actix_web::{
    get, http, middleware::Logger, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use env_logger::Env;
use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject,
};
use serde_json::json;
use tracing::{debug, error, info, warn};

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .json(json!({"message": "ok"}))
}

#[post("/mutate")]
async fn handle_mutate(
    reqst: HttpRequest,
    body: web::Json<AdmissionReview<DynamicObject>>,
) -> impl Responder {
    info!(
        "request recieved: method={:?}, uri={}",
        reqst.method(),
        reqst.uri(),
    );

    if let Some(content_type) = reqst.head().headers.get("content-type") {
        if content_type != "application/json" {
            let msg = format!("invalid content-type: {:?}", content_type);
            warn!("Warn: {}, Code: {}", msg, http::StatusCode::BAD_REQUEST);
            return HttpResponse::BadRequest().json(msg);
        }
    }

    let req: AdmissionRequest<_> = match body.into_inner().try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return HttpResponse::InternalServerError()
                .json(&AdmissionResponse::invalid(err.to_string()).into_review());
        }
    };

    let resp = AdmissionResponse::from(&req);

    HttpResponse::Ok().json(&resp.into_review())
}

#[actix_web::main]
async fn main() -> Result<(), anyhow::Error> {
    info!("Started http server: 0.0.0.0:8080");

    //let mut config = ServerConfig::new(NoClientAuth::new());
    //let cert_file = &mut BufReader::new(File::open("./certs/serverCert.pem")?);
    //let key_file = &mut BufReader::new(File::open("./certs/serverKey.pem")?);
    //let cert_chain = certs(cert_file).expect("error in cert");
    //let mut keys = rsa_private_keys(key_file).expect("error in key");
    //config.set_single_cert(cert_chain, keys.remove(0))?;

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(health)
            .service(handle_mutate)
    })
    //.bind_rustls("0.0.0.0:8443")?
    .bind("0.0.0.0:8443")?
    .run()
    .await?;

    Ok(())
}
