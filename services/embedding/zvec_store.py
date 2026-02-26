#!/usr/bin/env python3
"""zvec store for embedding service."""

import os
import shutil
from pathlib import Path

import numpy as np
import polars as pl
from sentence_transformers import SentenceTransformer
from tqdm import tqdm
import zvec

import embedding_pb2
from embedding_pb2 import SearchResultItem


EMBEDDING_DIM = 384
BATCH_SIZE = 100


class ZvecStore:
    """zvec-based vector store for category embeddings."""

    def __init__(
        self,
        data_dir: str = "./data",
        zvec_dir: str = "./data/embedding_store/zvec",
        model_name: str = "sentence-transformers/all-MiniLM-L12-v2",
    ):
        """Initialize the zvec store.

        Args:
            data_dir: Path to data directory containing parquet files.
            zvec_dir: Path to store zvec collections.
            model_name: Sentence transformer model name.
        """
        self.data_dir = Path(data_dir)
        self.zvec_dir = Path(zvec_dir)
        self.model_name = model_name

        self._model = None
        self._collections: dict[str, zvec.Collection] = {}

    @property
    def model(self) -> SentenceTransformer:
        """Lazy-load the sentence transformer model."""
        if self._model is None:
            self._model = SentenceTransformer(self.model_name)
        return self._model

    def _collection_path(self, wiki: str) -> Path:
        """Get the path for a wiki's collection."""
        return self.zvec_dir / f"{wiki}-categories"

    def _get_collection(self, wiki: str) -> zvec.Collection:
        """Get or open a collection for a wiki."""
        if wiki in self._collections:
            return self._collections[wiki]

        col_path = self._collection_path(wiki)

        if col_path.exists():
            col = zvec.open(str(col_path))
        else:
            col = self._create_collection(wiki)

        self._collections[wiki] = col
        return col

    def _create_collection(self, wiki: str) -> zvec.Collection:
        """Create a new collection for a wiki."""
        col_path = self._collection_path(wiki)
        self.zvec_dir.mkdir(parents=True, exist_ok=True)

        hnsw = zvec.HnswIndexParam(
            m=16,
            ef_construction=200,
            metric_type=zvec.MetricType.COSINE,
        )
        schema = zvec.CollectionSchema(
            name=f"{wiki}-categories",
            fields=[
                zvec.FieldSchema("qid", zvec.DataType.UINT32),
                zvec.FieldSchema("page_title", zvec.DataType.STRING),
            ],
            vectors=zvec.VectorSchema(
                "embedding", zvec.DataType.VECTOR_FP32, EMBEDDING_DIM, index_param=hnsw
            ),
        )

        return zvec.create_and_open(path=str(col_path), schema=schema)

    def injest(self, wiki: str, parquet_path: str | None = None) -> int:
        """Ingest categories from parquet for a wiki.

        Args:
            wiki: Wiki name (e.g., 'enwiki').
            parquet_path: Explicit path to the parquet file. If not given,
                          defaults to <data_dir>/<wiki>/categories.parquet.

        Returns:
            Number of records processed.
        """
        path = (
            Path(parquet_path)
            if parquet_path
            else self.data_dir / wiki / "categories.parquet"
        )

        if not path.exists():
            raise FileNotFoundError(f"Parquet file not found: {path}")

        df = pl.scan_parquet(path).select(["qid", "page_title"]).collect()

        total_records = len(df)
        print(f"Found {total_records} records to process for {wiki}")

        col = self._get_collection(wiki)

        records = []
        processed = 0

        for row in tqdm(
            df.iter_rows(named=True), total=total_records, desc=f"Indexing {wiki}"
        ):
            qid = row["qid"]
            page_title = row["page_title"]

            if qid is None or page_title is None:
                continue

            records.append((qid, page_title))

            if len(records) >= BATCH_SIZE:
                self._insert_batch(col, records)
                processed += len(records)
                records = []

        if records:
            self._insert_batch(col, records)
            processed += len(records)

        col.optimize()
        print(f"Indexed {processed} records for {wiki}")

        return processed

    def _insert_batch(self, col: zvec.Collection, records: list[tuple[int, str]]):
        """Insert a batch of records into the collection."""
        if not records:
            return

        titles = [title for _, title in records]
        embeddings = self.model.encode(titles)

        docs = []
        for (qid, title), embedding in zip(records, embeddings):
            doc = zvec.Doc(
                id=str(qid),
                fields={"qid": qid, "page_title": title},
                vectors={"embedding": embedding.tolist()},
            )
            docs.append(doc)

        col.insert(docs)

    def search(self, query: str, wiki: str, limit: int = 10) -> list[SearchResultItem]:
        """Search for categories matching a query.

        Args:
            query: Search query string.
            wiki: Wiki name.
            limit: Maximum number of results.

        Returns:
            List of search results.
        """
        col = self._get_collection(wiki)

        query_embedding = self.model.encode([query])[0]

        results = col.query(
            vectors=zvec.VectorQuery(
                field_name="embedding", vector=query_embedding.tolist()
            ),
            topk=limit,
        )

        search_results = []
        for r in results:
            fields = r.fields
            search_results.append(
                SearchResultItem(
                    score=r.score,
                    qid=fields.get("qid", 0),
                    page_title=fields.get("page_title", ""),
                )
            )

        return search_results
