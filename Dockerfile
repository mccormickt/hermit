FROM envoyproxy/envoy:1.24.0
ENTRYPOINT /usr/local/bin/envoy -c /etc/envoy.yaml -l debug --service-cluster proxy
