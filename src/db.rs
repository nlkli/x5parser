use crate::{error::Result, parser::models::pyaterochka::{StoreInfo, CatalogInfoWithTime}};
use rusqlite::Connection;
use std::sync::{Arc, LazyLock, OnceLock, Mutex};

static DB_PATH: OnceLock<String> = OnceLock::new();

pub fn init(path: Option<&str>) -> &String {
    DB_PATH.get_or_init(|| {
        path.unwrap_or("database.sqlite").into()
    })
}

static CONN: LazyLock<Arc<Mutex<Connection>>> = LazyLock::new(|| {
    let conn = Connection::open(init(None)).unwrap();
    conn.execute_batch(
        r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS pyaterochka_stores (
            id TEXT PRIMARY KEY,
            address TEXT,
            city TEXT,
            inserted_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS pyaterochka_products (
            id TEXT PRIMARY KEY,
            name TEXT,
            category TEXT,
            brand TEXT,
            rating REAL,
            rates_count INTEGER,
            image TEXT,
            property TEXT,
            updated_at INTEGER,
            inserted_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS pyaterochka_product_price_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            store_id TEXT,
            product_id TEXT,
            price REAL,
            card_price REAL,
            inserted_at INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_pph_store_id ON pyaterochka_product_price_history(store_id);
        CREATE INDEX IF NOT EXISTS idx_pph_product_id ON pyaterochka_product_price_history(product_id);
        COMMIT;
        "#,
    )
    .expect("Failed to execute batch");
    Arc::new(Mutex::new(conn))
});

pub fn pyaterochka_insert_data(store_info: &StoreInfo, catalogs: &[CatalogInfoWithTime]) -> Result<()> {
    let mut conn = CONN.lock().unwrap();
    let tx = conn.transaction()?;
    let now = chrono::Utc::now().timestamp();

    tx.execute(
        "INSERT OR IGNORE INTO pyaterochka_stores (id, address, city, inserted_at) VALUES (?1, ?2, ?3, ?4)",
        (&store_info.id, &store_info.address, &store_info.city, &now),
    )?;

    {
        let mut stmt_insert_product = tx.prepare(
            r#"INSERT INTO pyaterochka_products (
                id,
                name,
                category,
                brand,
                rating,
                rates_count,
                image,
                property,
                updated_at,
                inserted_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                name        = excluded.name,
                category    = excluded.category,
                brand       = excluded.brand,
                rating      = excluded.rating,
                rates_count = excluded.rates_count,
                image       = excluded.image,
                property    = excluded.property,
                updated_at  = excluded.updated_at"#
        )?;

        let mut stmt_insert_product_price_history = tx.prepare(
            r#"INSERT INTO pyaterochka_product_price_history (store_id, product_id, price, card_price, inserted_at)
            SELECT ?1, ?2, ?3, ?4, ?5
            WHERE NOT EXISTS (
                SELECT 1
                FROM pyaterochka_product_price_history p
                WHERE p.store_id = ?1
                  AND p.product_id = ?2
                  AND p.inserted_at = (
                      SELECT inserted_at
                      FROM pyaterochka_product_price_history
                      WHERE store_id = ?1 AND product_id = ?2
                      ORDER BY inserted_at DESC
                      LIMIT 1
                  )
                  AND p.price = ?3
                  AND p.card_price = ?4
            )"#
        )?;

        for c in catalogs.iter() {
            for p in c.info.products.iter() {
                let brand = c.info.brand_list.iter().find(|v| p.name.contains(*v));
                stmt_insert_product.execute((
                    &p.id,
                    &p.name,
                    &c.info.name,
                    brand,
                    &p.rating,
                    &p.rates_count,
                    &p.image,
                    &p.property,
                    &c.time,
                    &c.time,
                ))?;
                stmt_insert_product_price_history.execute((
                    &store_info.id,
                    &p.id,
                    &p.price,
                    &p.card_price,
                    &c.time,
                ))?;
            }
        }
    }

    tx.commit()?;

    Ok(())
}

// pub fn push_pyaterochka_products_batch(store_info: &StoreInfo, products: &[StdProduct]) -> Result<()> {
//     let mut conn = CONN.lock().unwrap();
//     let tx = conn.transaction()?;
//     let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
// 
//     tx.execute(
//         "INSERT OR IGNORE INTO pyaterochka_stores (id, address, city, inserted_at) VALUES (?1, ?2, ?3, ?4)",
//         (&store_info.id, &store_info.address, &store_info.city, &now),
//     )?;
// 
//     {
//         let mut stmt_insert_product = tx.prepare(
//             "INSERT OR IGNORE INTO pyaterochka_products (id, inserted_at) VALUES (?1, ?2)",
//         )?;
//         let mut stmt_exists = tx.prepare(
//             "SELECT 1 FROM pyaterochka_store_products WHERE id = ?1",
//         )?;
//         let mut stmt_insert_sp = tx.prepare(
//             "INSERT INTO pyaterochka_store_products (id, store_id, product_id, data, inserted_at, updated_at)
//              VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
//         )?;
//         let mut stmt_old_data = tx.prepare(
//             "SELECT data FROM pyaterochka_store_products WHERE id = ?1",
//         )?;
//         let mut stmt_history = tx.prepare(
//             "INSERT INTO pyaterochka_store_product_history (store_product_id, store_id, product_id, field, data, inserted_at)
//              VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
//         )?;
//         let mut stmt_update = tx.prepare(
//             "UPDATE pyaterochka_store_products SET data = ?1, updated_at = ?2 WHERE id = ?3",
//         )?;
// 
//         for product in products {
//             if product.price == 0. || product.card_price == 0. {
//                 continue;
//             }
//             let product_id = product.id.as_str();
//             let store_product_id = format!("{}_{product_id}", store_info.id);
//             let product_data = serde_json::to_string(product)?;
// 
//             stmt_insert_product.execute((product_id, &now))?;
// 
//             let exists = stmt_exists
//                 .query_row((&store_product_id,), |_| Ok(true))
//                 .optional()?
//                 .is_some();
// 
//             if !exists {
//                 stmt_insert_sp.execute((
//                     &store_product_id,
//                     &store_info.id,
//                     product_id,
//                     &product_data,
//                     &now,
//                     &now,
//                 ))?;
//                 let price_info = serde_json::to_string(&serde_json::json!({
//                     "price": product.price,
//                     "card_price": product.card_price
//                 }))?;
//                 stmt_history.execute((&store_product_id, &store_info.id, product_id, "price", &price_info, &now))?;
//             } else {
//                 let old_data: String = stmt_old_data.query_row((&store_product_id,), |r| r.get(0))?;
//                 let old_product: StdProduct = serde_json::from_str(&old_data)?;
// 
//                 if old_product.price != product.price || old_product.card_price != product.card_price {
//                     let price_info = serde_json::to_string(&serde_json::json!({
//                         "price": product.price,
//                         "card_price": product.card_price
//                     }))?;
//                     stmt_history.execute((&store_product_id, &store_info.id, product_id, "price", &price_info, &now))?;
//                 }
// 
//                 stmt_update.execute((&product_data, &now, &store_product_id))?;
//             }
//         }
//     }
// 
//     tx.commit()?;
//     Ok(())
// }
