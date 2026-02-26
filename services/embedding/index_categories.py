#!/usr/bin/env python3
"""CLI tool to index wiki categories into zvec."""

import argparse
import os
import sys

import grpc

import embedding_pb2
import embedding_pb2_grpc


def main():
    parser = argparse.ArgumentParser(description="Index wiki categories into zvec")
    parser.add_argument("--wiki", default="enwiki", help="Wiki to index")
    parser.add_argument(
        "--server",
        default=os.getenv("EMBEDDING_SERVER", "localhost:50051"),
        help="gRPC server address",
    )
    args = parser.parse_args()

    channel = grpc.insecure_channel(args.server)
    stub = embedding_pb2_grpc.EmbeddingServiceStub(channel)

    print(f"Indexing {args.wiki} categories via {args.server}...")

    try:
        resp = stub.Injest(embedding_pb2.InjestRequest(wiki=args.wiki))
        print(f"Indexed {resp.records_processed} records")
    except grpc.RpcError as e:
        print(f"Error: {e.code()}: {e.details()}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
