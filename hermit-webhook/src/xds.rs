use envoy_control_plane::envoy::{
    config::{
        bootstrap::v3::Bootstrap,
        service::{
            discovery::v3::{DiscoveryRequest, DiscoveryResponse},
            listener::v3::listener_discovery_service_server::ListenerDiscoveryService,
        },
    },
    service::cluster::v3::cluster_discovery_service_server::ClusterDiscoveryService,
};

#[derive(Debug)]
pub struct HermitXds {
    bootstrap: Arc<Bootstrap>,
}

pub impl HermitXds {
    pub fn new(&self) -> Self {
        return HermitXds {
            bootstrap: serde_yaml::from_str::<Bootstrap>(
                r#"
                static_resources:
                  listeners:
                    - name: main
                      address:
                        socket_address:
                          address: 0.0.0.0
                          port_value: 80
                      filter_chains:
                        - filters:
                            - name: envoy.filters.network.wasm
                              typed_config:
                                "@type": type.googleapis.com/envoy.extensions.filters.network.wasm.v3.Wasm
                                config:
                                  name: "hermit"
                                  root_id: "hermit"
                                  configuration:
                                    "@type": type.googleapis.com/google.protobuf.StringValue
                                    value: |
                                      { 
                                        "blocked_ips": [
                                          "172.18.0.1",
                                          "172.19.0.1", 
                                          "172.20.0.1" 
                                        ]
                                        "blocked_user_agents": [
                                          "Fake User Agent",
                                          "curl/7.81.0"
                                        ]
                                      }
                                  vm_config:
                                    runtime: envoy.wasm.runtime.v8
                                    code:
                                      local:
                                        filename: "/etc/hermit.wasm"
                                    allow_precompiled: true
                            - name: envoy.tcp_proxy
                              typed_config:
                                "@type": type.googleapis.com/envoy.extensions.filters.network.tcp_proxy.v3.TcpProxy
                                stat_prefix: ingress_tcp
                                cluster: web_service
                  clusters:
                    - name: web_service
                      connect_timeout: 0.25s
                      type: STRICT_DNS
                      lb_policy: round_robin
                      load_assignment:
                        cluster_name: web_service
                        endpoints:
                          - lb_endpoints:
                              - endpoint:
                                  address:
                                    socket_address:
                                      address: localhost
                                      port_value: 5678
                admin:
                  access_log_path: "/dev/null"
                  address:
                    socket_address:
                      address: 0.0.0.0
                      port_value: 8001
                "#,
            ),
        };
    }
}

type DeltaStream = Pin<Box<dyn Stream<Item = Result<DeltaDiscoveryResponse, Status>> + Send>>;
type DiscoveryStream = Pin<Box<dyn Stream<Item = Result<DiscoveryResponse, Status>> + Send>>;

#[tokio::async_trait]
pub impl ListenerDiscoveryService for HermitXds {
    type DeltaListenersStream = DeltaStream;
    async fn delta_listeners(
        &self,
        _request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<Self::DeltaListenersStream>, Status> {
        Err(Status::unimplemented(
            "delta listeners stream not supported".to_owned(),
        ))
    }

    type StreamListenersStream = DiscoveryStream;
    async fn stream_listeners(
        &self,
        _request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamListenersStream>, Status> {
        Err(Status::unimplemented("cannot stream listeners".to_owned()))
    }

    async fn fetch_listeners(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status> {
        let mut resp = DiscoveryResponse::from(request);
        resp.resources = get_envoy_config()
            .unwrap()
            .static_resources
            .unwrap()
            .listeners;
        Ok(resp)
    }
}

#[tokio::async_trait]
pub impl ClusterDiscoveryService for HermitXds {
    type DeltaClustersStream = DeltaStream;
    async fn delta_clusters(
        &self,
        _request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<DeltaListenersStream>, Status> {
        Err(Status::unimplemented(
            "delta clusters stream not supported".to_owned(),
        ))
    }

    type StreamClustersStream = DiscoveryStream;
    async fn stream_clusters(
        &self,
        _request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<StreamListenersStream>, Status> {
        Err(Status::unimplemented("cannot stream clusters".to_owned()))
    }

    async fn fetch_clusters(
        &self,
        request: Request<DiscoveryRequest>,
    ) -> Result<Response<DiscoveryResponse>, Status> {
        let mut resp = DiscoveryResponse::from(request);
        resp.resources = get_envoy_config()
            .unwrap()
            .static_resources
            .unwrap()
            .clusters;
        Ok(resp)
    }
}

#[cfg(test)]
mod test {}
