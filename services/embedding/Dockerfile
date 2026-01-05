FROM python:3.12-slim

WORKDIR /app

# Install system dependencies including curl for uv installation
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install uv
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin:$PATH"

# Copy project files
COPY pyproject.toml .
COPY embedding.proto .
COPY embedding_server.py .

# Install dependencies with uv
RUN uv sync --no-dev

# Generate Python protobuf code
RUN uv run python -m grpc_tools.protoc \
    -I. \
    --python_out=. \
    --grpc_python_out=. \
    embedding.proto

# Download model at build time (optional, speeds up startup)
RUN uv run python -c "from sentence_transformers import SentenceTransformer; SentenceTransformer('sentence-transformers/all-MiniLM-L12-v2')"

EXPOSE 50051

CMD ["uv", "run", "python", "embedding_server.py"]
