.PHONY: proto client clean docker sync

# Sync dependencies with uv
sync:
	uv sync

# Generate protobuf code
proto: sync
	uv run python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. embedding.proto

# Run the server — use the root Makefile target to ensure correct DATA_DIR/ZVEC_DIR
server:
	$(MAKE) -C ../.. embedding-server

# Build and run with Docker
docker-build:
	docker build -t embedding-service .

docker-run:
	docker run -p 50051:50051 embedding-service

# Docker Compose
compose-up:
	docker-compose up --build

compose-down:
	docker-compose down

# Clean generated files
clean:
	rm -f embedding_pb2.py embedding_pb2_grpc.py
	rm -rf __pycache__ .venv

# Run tests
test: proto
	uv run embedding_client.py
