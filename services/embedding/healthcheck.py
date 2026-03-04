#!/usr/bin/env python3
"""Check that the embedding gRPC server is reachable. Exits non-zero if not."""

import os
import sys

import grpc

import embedding_pb2
import embedding_pb2_grpc


def main():
    server = os.getenv("EMBEDDING_SERVER", "localhost:50051")
    try:
        channel = grpc.insecure_channel(server)
        stub = embedding_pb2_grpc.EmbeddingServiceStub(channel)
        stub.HealthCheck(embedding_pb2.HealthCheckRequest(), timeout=3)
    except grpc.RpcError:
        print(
            f"Error: embedding server not reachable at {server}."
            " Run 'make embedding-server' first.",
            file=sys.stderr,
        )
        sys.exit(1)


if __name__ == "__main__":
    main()
