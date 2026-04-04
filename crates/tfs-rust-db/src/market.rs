use crate::pool::DbPool;
use sqlx::FromRow;
use tfs_rust_common::error::{Result, TfsRustError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketOfferType {
    Buy = 1,
    Sell = 2,
}

#[derive(Debug, Clone)]
pub struct MarketOffer {
    pub amount: u16,
    pub price: u32,
    pub timestamp: u64,
    pub counter: u16,
    pub item_id: Option<u16>,
    pub player_name: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketOfferRecord {
    pub id: u32,
    pub player_id: u32,
    pub sale: i8,
    pub itemtype: u16,
    pub amount: u16,
    pub created: u32,
    pub anonymous: i8,
    pub price: u64,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketHistoryRecord {
    pub id: u32,
    pub player_id: u32,
    pub sale: i8,
    pub itemtype: u16,
    pub amount: u16,
    pub price: u64,
    pub expires_at: u32,
    pub inserted: u32,
    pub state: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct HistoryInsert {
    pub player_id: u32,
    pub sale: MarketOfferType,
    pub itemtype: u16,
    pub amount: u16,
    pub price: u32,
    pub expires_at: u64,
    pub state: u8,
}

pub struct MarketStore<'a> {
    pool: &'a DbPool,
}

impl<'a> MarketStore<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn fetch_offers(&self) -> Result<Vec<MarketOfferRecord>> {
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                async move {
                    sqlx::query_as::<_, MarketOfferRecord>(
                        "SELECT id, player_id, sale, itemtype, amount, created, anonymous, price FROM market_offers",
                    )
                    .fetch_all(&pool)
                    .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    // C++ reference: src/iomarket.cpp IOMarket::getActiveOffers
    pub async fn get_active_offers(
        &self,
        sale: MarketOfferType,
        itemtype: u16,
        market_offer_duration_secs: u64,
    ) -> Result<Vec<MarketOffer>> {
        #[derive(Debug, FromRow)]
        struct ActiveOfferRow {
            id: u32,
            amount: u16,
            price: u32,
            created: u64,
            anonymous: i8,
            player_name: Option<String>,
        }

        let sql = "SELECT id, amount, price, created, anonymous, (SELECT name FROM players WHERE id = player_id) AS player_name FROM market_offers WHERE sale = ? AND itemtype = ?";
        let rows = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query_as::<_, ActiveOfferRow>(&sql)
                        .bind(sale as i8)
                        .bind(itemtype)
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| MarketOffer {
                amount: row.amount,
                price: row.price,
                timestamp: row.created + market_offer_duration_secs,
                counter: (row.id & 0xFFFF) as u16,
                item_id: None,
                player_name: Some(if row.anonymous == 0 {
                    row.player_name.unwrap_or_default()
                } else {
                    "Anonymous".to_string()
                }),
            })
            .collect())
    }

    // C++ reference: src/iomarket.cpp IOMarket::getOwnOffers
    pub async fn get_own_offers(
        &self,
        sale: MarketOfferType,
        player_id: u32,
        market_offer_duration_secs: u64,
    ) -> Result<Vec<MarketOffer>> {
        #[derive(Debug, FromRow)]
        struct OwnOfferRow {
            id: u32,
            amount: u16,
            price: u32,
            created: u64,
            itemtype: u16,
        }

        let sql = "SELECT id, amount, price, created, itemtype FROM market_offers WHERE player_id = ? AND sale = ?";
        let rows = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query_as::<_, OwnOfferRow>(&sql)
                        .bind(player_id)
                        .bind(sale as i8)
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| MarketOffer {
                amount: row.amount,
                price: row.price,
                timestamp: row.created + market_offer_duration_secs,
                counter: (row.id & 0xFFFF) as u16,
                item_id: Some(row.itemtype),
                player_name: None,
            })
            .collect())
    }

    pub async fn create_offer(
        &self,
        player_id: u32,
        sale: MarketOfferType,
        itemtype: u16,
        amount: u16,
        price: u64,
    ) -> Result<()> {
        let sql = "INSERT INTO market_offers (player_id, sale, itemtype, amount, created, anonymous, price) VALUES (?, ?, ?, ?, ?, ?, ?)";
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| TfsRustError::Database(e.to_string()))?
            .as_secs();
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query(&sql)
                        .bind(player_id)
                        .bind(sale as i8)
                        .bind(itemtype)
                        .bind(amount)
                        .bind(created)
                        .bind(0i8)
                        .bind(price)
                        .execute(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    // C++ reference: src/iomarket.cpp IOMarket::acceptOffer
    pub async fn accept_offer(&self, offer_id: u32, amount: u16) -> Result<()> {
        let sql = "UPDATE market_offers SET amount = amount - ? WHERE id = ?";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query(&sql)
                        .bind(amount)
                        .bind(offer_id)
                        .execute(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    // C++ reference: src/iomarket.cpp IOMarket::deleteOffer
    pub async fn cancel_offer(&self, offer_id: u32) -> Result<()> {
        let sql = "DELETE FROM market_offers WHERE id = ?";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move { sqlx::query(&sql).bind(offer_id).execute(&pool).await }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    // C++ reference: src/iomarket.cpp IOMarket::appendHistory
    pub async fn append_history(&self, input: HistoryInsert) -> Result<()> {
        let inserted = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| TfsRustError::Database(e.to_string()))?
            .as_secs();
        let sql = "INSERT INTO market_history (player_id, sale, itemtype, amount, price, expires_at, inserted, state) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query(&sql)
                        .bind(input.player_id)
                        .bind(input.sale as i8)
                        .bind(input.itemtype)
                        .bind(input.amount)
                        .bind(input.price)
                        .bind(input.expires_at)
                        .bind(inserted)
                        .bind(input.state)
                        .execute(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(())
    }

    // C++ reference: src/iomarket.cpp IOMarket::moveOfferToHistory
    pub async fn move_offer_to_history(
        &self,
        offer_id: u32,
        state: u8,
        market_offer_duration_secs: u64,
    ) -> Result<bool> {
        #[derive(Debug, FromRow)]
        struct OfferSnapshot {
            player_id: u32,
            sale: i8,
            itemtype: u16,
            amount: u16,
            price: u32,
            created: u64,
        }

        let select_sql =
            "SELECT player_id, sale, itemtype, amount, price, created FROM market_offers WHERE id = ?";
        let maybe_offer = self
            .pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let select_sql = select_sql.to_string();
                async move {
                    sqlx::query_as::<_, OfferSnapshot>(&select_sql)
                        .bind(offer_id)
                        .fetch_optional(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        let Some(offer) = maybe_offer else {
            return Ok(false);
        };

        let delete_sql = "DELETE FROM market_offers WHERE id = ?";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let delete_sql = delete_sql.to_string();
                async move { sqlx::query(&delete_sql).bind(offer_id).execute(&pool).await }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        let sale = if offer.sale == MarketOfferType::Buy as i8 {
            MarketOfferType::Buy
        } else {
            MarketOfferType::Sell
        };

        self.append_history(HistoryInsert {
            player_id: offer.player_id,
            sale,
            itemtype: offer.itemtype,
            amount: offer.amount,
            price: offer.price,
            expires_at: offer.created + market_offer_duration_secs,
            state,
        })
        .await?;

        Ok(true)
    }

    pub async fn browse_by_item(&self, itemtype: u16) -> Result<Vec<MarketOfferRecord>> {
        let sql = "SELECT id, player_id, sale, itemtype, amount, created, anonymous, price FROM market_offers WHERE itemtype = ? ORDER BY created DESC";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query_as::<_, MarketOfferRecord>(&sql)
                        .bind(itemtype)
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    pub async fn browse_own_offers(&self, player_id: u32) -> Result<Vec<MarketOfferRecord>> {
        let sql = "SELECT id, player_id, sale, itemtype, amount, created, anonymous, price FROM market_offers WHERE player_id = ? ORDER BY created DESC";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query_as::<_, MarketOfferRecord>(&sql)
                        .bind(player_id)
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }

    pub async fn browse_history(&self, player_id: u32) -> Result<Vec<MarketHistoryRecord>> {
        let sql = "SELECT id, player_id, sale, itemtype, amount, price, expires_at, inserted, state FROM market_history WHERE player_id = ? ORDER BY inserted DESC";
        self.pool
            .execute_with_retry(|| {
                let pool = self.pool.inner().clone();
                let sql = sql.to_string();
                async move {
                    sqlx::query_as::<_, MarketHistoryRecord>(&sql)
                        .bind(player_id)
                        .fetch_all(&pool)
                        .await
                }
            })
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))
    }
}
