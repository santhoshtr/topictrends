#!/usr/bin/env python3
"""
Python client for the Embedding gRPC service.

Usage:
    python embedding_client.py

Make sure the server is running and you've generated the protobuf code:
    python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. embedding.proto
"""

import grpc
import numpy as np
from typing import List, Optional, Tuple
import logging

# Import generated protobuf code
import embedding_pb2
import embedding_pb2_grpc


class EmbeddingClient:
    """Client for interacting with the Embedding gRPC service."""

    def __init__(self, host: str = "localhost", port: int = 50051):
        """
        Initialize the client.

        Args:
            host: Server hostname
            port: Server port
        """
        self.address = f"{host}:{port}"
        self.channel = grpc.insecure_channel(
            self.address,
            options=[
                ("grpc.max_send_message_length", 100 * 1024 * 1024),
                ("grpc.max_receive_message_length", 100 * 1024 * 1024),
            ],
        )
        self.stub = embedding_pb2_grpc.EmbeddingServiceStub(self.channel)
        logging.info(f"Connected to embedding service at {self.address}")

    def health_check(self) -> Tuple[bool, str]:
        """
        Check if the service is healthy.

        Returns:
            Tuple of (healthy: bool, model_name: str)
        """
        try:
            request = embedding_pb2.HealthCheckRequest()
            response = self.stub.HealthCheck(request)
            return response.healthy, response.model_name
        except grpc.RpcError as e:
            logging.error(f"Health check failed: {e}")
            return False, ""

    def encode(self, texts: List[str], prompt_name: Optional[str] = None) -> np.ndarray:
        """
        Encode texts into embeddings.

        Args:
            texts: List of text strings to encode
            prompt_name: Optional prompt name (e.g., "query" for queries)

        Returns:
            numpy array of shape (len(texts), embedding_dim)
        """
        try:
            request = embedding_pb2.EncodeRequest(texts=texts)
            if prompt_name:
                request.prompt_name = prompt_name

            response = self.stub.Encode(request)

            # Convert protobuf embeddings to numpy array
            embeddings = np.array([list(emb.values) for emb in response.embeddings])

            logging.debug(f"Encoded {len(texts)} texts into shape {embeddings.shape}")
            return embeddings

        except grpc.RpcError as e:
            logging.error(f"Encode failed: {e.code()}: {e.details()}")
            raise

    def encode_queries(self, queries: List[str]) -> np.ndarray:
        """
        Encode queries with the appropriate prompt.

        Args:
            queries: List of query strings

        Returns:
            numpy array of query embeddings
        """
        return self.encode(queries, prompt_name="query")

    def encode_documents(self, documents: List[str]) -> np.ndarray:
        """
        Encode documents without a prompt.

        Args:
            documents: List of document strings

        Returns:
            numpy array of document embeddings
        """
        return self.encode(documents, prompt_name=None)

    def compute_similarity(
        self, query_embeddings: np.ndarray, document_embeddings: np.ndarray
    ) -> np.ndarray:
        """
        Compute similarity between query and document embeddings.

        Args:
            query_embeddings: numpy array of shape (num_queries, embedding_dim)
            document_embeddings: numpy array of shape (num_docs, embedding_dim)

        Returns:
            numpy array of shape (num_queries, num_docs) with similarity scores
        """
        try:
            # Convert numpy arrays to protobuf format
            request = embedding_pb2.SimilarityRequest()

            for query_emb in query_embeddings:
                emb_msg = request.query_embeddings.add()
                emb_msg.values.extend(query_emb.tolist())

            for doc_emb in document_embeddings:
                emb_msg = request.document_embeddings.add()
                emb_msg.values.extend(doc_emb.tolist())

            response = self.stub.ComputeSimilarity(request)

            # Reshape flat array into matrix
            similarity_matrix = np.array(response.similarities).reshape(
                response.num_queries, response.num_documents
            )

            logging.debug(
                f"Computed similarity matrix of shape {similarity_matrix.shape}"
            )
            return similarity_matrix

        except grpc.RpcError as e:
            logging.error(f"ComputeSimilarity failed: {e.code()}: {e.details()}")
            raise

    def find_most_similar(
        self, queries: List[str], documents: List[str], top_k: int = 1
    ) -> List[List[Tuple[int, float]]]:
        """
        Find the most similar documents for each query.

        Args:
            queries: List of query strings
            documents: List of document strings
            top_k: Number of top results to return per query

        Returns:
            List of lists, where each inner list contains (doc_index, score) tuples
        """
        # Encode queries and documents
        query_embs = self.encode_queries(queries)
        doc_embs = self.encode_documents(documents)

        # Compute similarity
        similarity_matrix = self.compute_similarity(query_embs, doc_embs)

        # Find top-k for each query
        results = []
        for query_idx in range(len(queries)):
            scores = similarity_matrix[query_idx]
            top_indices = np.argsort(scores)[::-1][:top_k]
            top_results = [(int(idx), float(scores[idx])) for idx in top_indices]
            results.append(top_results)

        return results

    def close(self):
        """Close the gRPC channel."""
        self.channel.close()
        logging.info("Connection closed")

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()


def main():
    """Example usage of the embedding client."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    )

    # Use context manager for automatic cleanup
    with EmbeddingClient() as client:
        # Health check
        print("=" * 60)
        print("Health Check")
        print("=" * 60)
        healthy, model_name = client.health_check()
        print(f"Service healthy: {healthy}")
        print(f"Model: {model_name}\n")

        # Define queries and documents
        queries = [
            "What is the capital of China?",
            "Explain gravity",
            "How do computers work?",
        ]

        documents = [
            "The capital of China is Beijing.",
            "Gravity is a force that attracts two bodies towards each other.",
            "Computers process information using electronic circuits and binary code.",
            "Beijing is a major city in China with a rich history.",
            "Newton discovered the law of universal gravitation.",
        ]

        # Example 1: Basic encoding
        print("=" * 60)
        print("Example 1: Basic Encoding")
        print("=" * 60)
        query_embeddings = client.encode_queries(queries)
        document_embeddings = client.encode_documents(documents)
        print(f"Query embeddings shape: {query_embeddings.shape}")
        print(f"Document embeddings shape: {document_embeddings.shape}\n")

        # Example 2: Compute similarity
        print("=" * 60)
        print("Example 2: Similarity Matrix")
        print("=" * 60)
        similarity_matrix = client.compute_similarity(
            query_embeddings, document_embeddings
        )
        print(f"Similarity matrix shape: {similarity_matrix.shape}")
        print("\nSimilarity scores:")
        print(similarity_matrix)
        print()

        # Example 3: Find most similar documents
        print("=" * 60)
        print("Example 3: Find Most Similar Documents")
        print("=" * 60)
        results = client.find_most_similar(queries, documents, top_k=2)

        for i, query in enumerate(queries):
            print(f'\nQuery: "{query}"')
            print("Top matches:")
            for rank, (doc_idx, score) in enumerate(results[i], 1):
                print(f'  {rank}. "{documents[doc_idx]}" (score: {score:.4f})')

        # Example 4: Batch processing
        print("\n" + "=" * 60)
        print("Example 4: Batch Processing")
        print("=" * 60)
        large_batch = [f"Sample text number {i}" for i in range(100)]
        batch_embeddings = client.encode_documents(large_batch)
        print(f"Encoded {len(large_batch)} texts")
        print(f"Embeddings shape: {batch_embeddings.shape}")


if __name__ == "__main__":
    main()
