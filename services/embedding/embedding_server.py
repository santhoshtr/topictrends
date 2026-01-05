#!/usr/bin/env python3
"""
gRPC server for vector embeddings using Qwen3-Embedding model.

Installation:
    pip install grpcio grpcio-tools sentence-transformers transformers

Generate Python code from proto:
    python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. embedding.proto
"""

import grpc
from concurrent import futures
import logging
import signal
import sys
import numpy as np
from sentence_transformers import SentenceTransformer

# Import generated protobuf code
import embedding_pb2
import embedding_pb2_grpc


class EmbeddingServicer(embedding_pb2_grpc.EmbeddingServiceServicer):
    """gRPC servicer for embedding operations."""

    def __init__(self, model_name: str = "sentence-transformers/all-MiniLM-L12-v2"):
        """Initialize the embedding model."""
        logging.info(f"Loading model: {model_name}")
        self.model = SentenceTransformer(model_name)
        self.model_name = model_name
        logging.info("Model loaded successfully")

    def Encode(self, request, context):
        """Encode texts into embeddings."""
        try:
            if not request.texts:
                context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
                context.set_details("texts field cannot be empty")
                return embedding_pb2.EncodeResponse()

            # Get prompt_name if provided
            prompt_name = (
                request.prompt_name if request.HasField("prompt_name") else None
            )

            # Encode texts
            logging.debug(
                f"Encoding {len(request.texts)} texts with prompt_name={prompt_name}"
            )
            embeddings = self.model.encode(request.texts, prompt_name=prompt_name)

            # Convert numpy arrays to protobuf format
            response = embedding_pb2.EncodeResponse()
            for embedding in embeddings:
                emb_msg = response.embeddings.add()
                emb_msg.values.extend(embedding.tolist())

            logging.debug(f"Successfully encoded {len(response.embeddings)} embeddings")
            return response

        except Exception as e:
            logging.error(f"Error in Encode: {str(e)}", exc_info=True)
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(f"Encoding failed: {str(e)}")
            return embedding_pb2.EncodeResponse()

    def ComputeSimilarity(self, request, context):
        """Compute similarity between query and document embeddings."""
        try:
            if not request.query_embeddings or not request.document_embeddings:
                context.set_code(grpc.StatusCode.INVALID_ARGUMENT)
                context.set_details(
                    "Both query_embeddings and document_embeddings must be provided"
                )
                return embedding_pb2.SimilarityResponse()

            # Convert protobuf embeddings to numpy arrays
            query_embs = np.array(
                [list(emb.values) for emb in request.query_embeddings]
            )
            doc_embs = np.array(
                [list(emb.values) for emb in request.document_embeddings]
            )

            logging.debug(
                f"Computing similarity: {query_embs.shape} x {doc_embs.shape}"
            )

            # Compute similarity using the model
            similarity_matrix = self.model.similarity(query_embs, doc_embs)

            # Convert to numpy if needed and flatten
            if hasattr(similarity_matrix, "cpu"):
                similarity_matrix = similarity_matrix.cpu().numpy()
            similarities_flat = similarity_matrix.flatten().tolist()

            response = embedding_pb2.SimilarityResponse(
                similarities=similarities_flat,
                num_queries=len(request.query_embeddings),
                num_documents=len(request.document_embeddings),
            )

            logging.debug("Similarity computation successful")
            return response

        except Exception as e:
            logging.error(f"Error in ComputeSimilarity: {str(e)}", exc_info=True)
            context.set_code(grpc.StatusCode.INTERNAL)
            context.set_details(f"Similarity computation failed: {str(e)}")
            return embedding_pb2.SimilarityResponse()

    def HealthCheck(self, request, context):
        """Health check endpoint."""
        return embedding_pb2.HealthCheckResponse(
            healthy=True, model_name=self.model_name
        )


def serve(port: int = 50051, max_workers: int = 10):
    """Start the gRPC server."""
    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=max_workers),
        options=[
            ("grpc.max_send_message_length", 100 * 1024 * 1024),  # 100MB
            ("grpc.max_receive_message_length", 100 * 1024 * 1024),  # 100MB
        ],
    )

    embedding_pb2_grpc.add_EmbeddingServiceServicer_to_server(
        EmbeddingServicer(), server
    )

    server.add_insecure_port(f"[::]:{port}")
    server.start()

    logging.info(f"Server started on port {port}")

    # Graceful shutdown
    def handle_sigterm(*_):
        logging.info("Received shutdown signal")
        done_event = server.stop(grace=10)
        done_event.wait()
        logging.info("Server stopped")
        sys.exit(0)

    signal.signal(signal.SIGTERM, handle_sigterm)
    signal.signal(signal.SIGINT, handle_sigterm)

    server.wait_for_termination()


if __name__ == "__main__":
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    )

    serve()
