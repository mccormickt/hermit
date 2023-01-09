use actix_web::{
    get, http, middleware::Logger, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use env_logger::Env;
use kube::{
    core::{
        admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
        DynamicObject,
    },
    ResourceExt,
};
use serde_json::json;
use tracing::{error, info, warn};

mod util;

#[get("/healthz")]
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

    let req: AdmissionRequest<_> = match body.into_inner().try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return HttpResponse::InternalServerError()
                .json(&AdmissionResponse::invalid(err.to_string()).into_review());
        }
    };

    let mut res = AdmissionResponse::from(&req);

    if let Some(obj) = req.object {
        let name = obj.name_any();
        info!("injecting envoy sidecar in pod {}", name);
        res = match util::inject_sidecar(res.clone(), &obj).await {
            Ok(res) => {
                info!("accepted: {:?} on Pod {}", req.operation, name);
                res
            }
            Err(err) => {
                warn!("denied: {:?} on {} ({})", req.operation, name, err);
                res.deny(err.to_string())
            }
        };
    };

    HttpResponse::Ok().json(&res.into_review())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    info!("Started http server: 0.0.0.0:8080");
    //let mut config = ServerConfig::new(NoClientAuth::new());
    //let cert_file = &mut BufReader::new(File::open("./certs/serverCert.pem")?);
    //let key_file = &mut BufReader::new(File::open("./certs/serverKey.pem")?);
    //let cert_chain = certs(cert_file).expect("error in cert");
    //let mut keys = rsa_private_keys(key_file).expect("error in key");
    //config.set_single_cert(cert_chain, keys.remove(0))?;

    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(health)
            .service(handle_mutate)
    })
    //.bind_rustls("0.0.0.0:8443")?
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
