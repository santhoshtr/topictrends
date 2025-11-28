use arrow_array::RecordBatch;
use arrow_array::{Float32Array, RecordBatchIterator, StringArray};
use futures::StreamExt;
use lancedb::{
    Connection,
    arrow::{
        RecordBatchStream,
        arrow_schema::{DataType, Field, Schema},
    },
    connect,
    query::{ExecutableQuery, QueryBase},
};
use std::{error::Error, sync::Arc};

async fn search(
    db: Connection,
    query: String,
    k: i32,
) -> Result<Vec<RecordBatch>, Box<dyn std::error::Error>> {
    let table_name = "category";

    let table = db.open_table(table_name).execute().await.unwrap();

    let stream: std::pin::Pin<Box<dyn RecordBatchStream + Send + 'static>> = table
        .query()
        .limit(2)
        .nearest_to(&[1.0; 128])?
        .execute()
        .await
        .unwrap();

    Ok(Vec::new())
}

pub async fn init_db(uri: String) -> Result<Connection, Box<dyn std::error::Error>> {
    let db = connect(&uri).execute().await?;
    let table_name = "category";

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        //Field::new("item", DataType::Utf8, true),
    ]));
    let table_name = "category";
    db.create_empty_table(table_name, schema).execute().await;
    Ok(db)
}
