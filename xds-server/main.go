package main

import (
	"context"
	"net"
	"os"

	"github.com/envoyproxy/go-control-plane/envoy/api/v2/core"
	core "github.com/envoyproxy/go-control-plane/envoy/config/core/v3"
	listener "github.com/envoyproxy/go-control-plane/envoy/config/listener/v3"
	listenerservice "github.com/envoyproxy/go-control-plane/envoy/service/listener/v3"
	"github.com/envoyproxy/go-control-plane/pkg/cache/types"
	"github.com/envoyproxy/go-control-plane/pkg/cache/v3"
	"github.com/envoyproxy/go-control-plane/pkg/resource/v3"
	xds "github.com/envoyproxy/go-control-plane/pkg/server/v3"
	"github.com/envoyproxy/go-control-plane/pkg/wellknown"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/anypb"
)

var (
	snapshotCache cache.SnapshotCache
)

func main() {
	snapshotCache = cache.NewSnapshotCache(false, cache.IDHash{}, nil)
	server := xds.NewServer(context.Background(), snapshotCache, nil)
	grpcServer := grpc.NewServer()
	lis, _ := net.Listen("tcp", ":8080")

	listenerservice.RegisterListenerDiscoveryServiceServer(grpcServer, server)

	hostname, _ := os.Hostname()
	snapshot := snapshotCache.SetSnapshot(context.Background(), hostname, &cache.Snapshot{})

}

func BuildSnapshot() *cache.Snapshot {
	snapshot, _ := cache.NewSnapshot("1",
		map[resource.Type][]types.Resource{
			resource.ClusterType:  {},
			resource.ListenerType: {makeListener()},
		},
	)
	return snapshot
}

func makeListener() *listener.Listener {
	routerConfig, _ := anypb.New(&router.Router{})
	pbst, err := anypb.New(manager)
	if err != nil {
		panic(err)
	}

	return &listener.Listener{
		Name: "main",
		Address: &core.Address{
			Address: &core.Address_SocketAddress{
				SocketAddress: &core.SocketAddress{
					Protocol: core.SocketAddress_TCP,
					Address:  "0.0.0.0",
					PortSpecifier: &core.SocketAddress_PortValue{
						PortValue: ListenerPort,
					},
				},
			},
		},
		FilterChains: []*listener.FilterChain{{
			Filters: []*listener.Filter{{
				Name: wellknown.HTTPConnectionManager,
				ConfigType: &listener.Filter_TypedConfig{
					TypedConfig: pbst,
				},
			}},
		}},
	}
}
