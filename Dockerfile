FROM envoyproxy/envoy:v1.24-latest
ENTRYPOINT /usr/local/bin/envoy -c /etc/envoy.yaml -l debug --service-cluster proxy
