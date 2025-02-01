use rusqlite::{params, Connection, Result};
use rweb::{self, reject, Rejection, Reply};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
struct Product {
    name: String,
    price: f64,
    quantity: String,
}

async fn init_db() -> Result<Connection> {
    let db_path = "products.db";

    // Check if the database file exists, if not create it
    if !Path::new(db_path).exists() {
        fs::File::create(db_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    }

    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS product (
                  id INTEGER PRIMARY KEY,
                  name TEXT NOT NULL,
                  price REAL NOT NULL,
                  quantity TEXT NOT NULL
                  )",
        [],
    )?;
    Ok(conn)
}

#[rweb::get("/products")]
async fn get_products(
    db: Arc<Mutex<Connection>>,
) -> Result<rweb::reply::Json<Vec<Product>>, Rejection> {
    let conn = db.lock().await;
    let mut stmt = conn
        .prepare("SELECT name, price, quantity FROM product")
        .map_err(|_| reject::not_found())?;
    let product_iter = stmt
        .query_map([], |row| {
            Ok(Product {
                name: row.get(0)?,
                price: row.get(1)?,
                quantity: row.get(2)?,
            })
        })
        .map_err(|_| reject::not_found())?;

    let mut products = Vec::new();
    for product in product_iter {
        products.push(product.map_err(|_| reject::not_found())?);
    }
    Ok(rweb::reply::json(&products))
}

#[rweb::post("/products")]
async fn submit_products(
    db: Arc<Mutex<Connection>>,
    new_products: Vec<Product>,
) -> Result<rweb::reply::Json<&'static str>, Rejection> {
    let conn = db.lock().await;
    conn.execute("DELETE FROM product", [])
        .map_err(|_| reject::not_found())?;
    for product in new_products {
        conn.execute(
            "INSERT INTO product (name, price, quantity) VALUES (?1, ?2, ?3)",
            params![product.name, product.price, product.quantity],
        )
        .map_err(|_| reject::not_found())?;
    }
    Ok(rweb::reply::json(&"Products updated"))
}

#[tokio::main]
async fn main() {
    let db = Arc::new(Mutex::new(init_db().await.unwrap()));

    // Combine endpoints and inject the DB as state.
    let api = rweb::endpoints![get_products, submit_products].with_state(db);

    println!("Listening on http://127.0.0.1:3030");
    rweb::start_server(api, ([127, 0, 0, 1], 3030)).await;
}
