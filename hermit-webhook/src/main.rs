use actix_web::{middleware::Logger, App, HttpServer};
use anyhow::Result;
use env_logger::Env;

use envoy_control_plane::envoy::service::listener::v3::listener_discovery_service_server::ListenerDiscoveryServiceServer;
use tonic::transport::Server;
use tracing::info;

mod admission;
mod xds;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    info!("Started http server: 0.0.0.0:8080");
    //let mut config = ServerConfig::new(NoClientAuth::new());
    //let cert_file = &mut BufReader::new(File::open("./certs/serverCert.pem")?);
    //let key_file = &mut BufReader::new(File::open("./certs/serverKey.pem")?);
    //let cert_chain = certs(cert_file).expect("error in cert");
    //let mut keys = rsa_private_keys(key_file).expect("error in key");
    //config.set_single_cert(cert_chain, keys.remove(0))?;

    // Start admission webhook server
    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(admission::health)
            .service(admission::handle_mutate)
    })
    //.bind_rustls("0.0.0.0:8443")?
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    // Start xds grpc server
    let server = xds::HermitXds::new();
    Server::builder()
        .add_service(ListenerDiscoveryServiceServer::new(server))
        .serve("0.0.0.0:9090")
        .await
        .unwrap()
}
