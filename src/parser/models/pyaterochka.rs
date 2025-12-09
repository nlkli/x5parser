use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CatalogInfoWithTime {
    pub info: CatalogInfo,
    pub time: i64,
}

impl CatalogInfoWithTime {
    pub fn from_catalog_with_id(c: Catalog, id: String, time: Option<i64>) -> Self {
        Self {
            info: CatalogInfo::from_catalog_with_id(c, id),
            time: time.unwrap_or(chrono::Utc::now().timestamp()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CatalogInfo {
    pub id: String,
    pub name: String,
    pub brand_list: Vec<String>,
    pub products: Vec<ProductInfo>,
}

impl CatalogInfo {
    pub fn from_catalog_with_id(mut c: Catalog, id: String) -> Self {
        let name = std::mem::take(&mut c.name);
        let filters = std::mem::take(&mut c.filters);
        let brand_list = filters.into_iter()
            .filter(|v| v.field_name == "brand")
            .map(|v| v.list_values.unwrap_or_default().all)
            .next()
            .unwrap_or_default();
        let products = c.products.into_iter()
            .map(Into::<ProductInfo>::into)
            .collect();
        Self { 
            id: id,
            name: name, 
            brand_list: brand_list, 
            products: products,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductInfo {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub card_price: f64,
    pub rating: Option<f64>,
    pub rates_count: Option<u32>,
    pub image: Option<String>,
    pub property: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct StoreInfo {
    pub id: String,
    pub address: String,
    pub city: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct StoreApiInfo {
    #[serde(default)]
    pub shop_address: String,

    #[serde(default)]
    pub store_city: String,

    #[serde(default)]
    pub sap_code: String,

    #[serde(default)]
    pub has_delivery: bool,

    #[serde(default)]
    pub has_24h_delivery: bool,
}

impl Into<StoreInfo> for StoreApiInfo {
    fn into(self) -> StoreInfo {
        return StoreInfo {
            id: self.sap_code,
            address: self.shop_address,
            city: self.store_city,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Catalog {
    pub name: String,

    #[serde(default)]
    pub filters: Vec<Filter>,

    pub products: Vec<Product>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Filter {
    pub field_name: String,
    pub filter_type: String,
    #[serde(default)]
    pub list_values: Option<FilterListValues>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct FilterListValues {
    #[serde(default)]
    pub all: Vec<String>,
}

/// Основная структура продукта
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Product {
    /// Уникальный код товара (PLU)
    pub plu: u64,

    /// Название товара
    #[serde(default)]
    pub name: String,

    /// Ссылки на изображения в разных размерах
    #[serde(default)]
    pub image_links: ImageLinks,

    /// Единица измерения (обычно "шт" или "кг")
    #[serde(default)]
    pub uom: String,

    /// Шаг количества для добавления в корзину
    #[serde(default)]
    pub step: String,

    /// Рейтинг товара (может отсутствовать)
    #[serde(default)]
    pub rating: Option<Rating>,

    /// Информация об акции
    #[serde(default)]
    pub promo: Option<serde_json::Value>,

    /// Цены товара
    pub prices: Prices,

    /// Метки/лейблы (например, скидка -12%)
    #[serde(default)]
    pub labels: Option<Vec<Label>>,

    /// Уточнение веса/объёма (например, "275 г")
    #[serde(default)]
    pub property_clarification: Option<String>,

    /// Признак возрастного ограничения
    #[serde(default)]
    pub has_age_restriction: bool,

    /// Максимальное количество в заказе
    #[serde(default)]
    pub stock_limit: Option<String>,

    /// Бейджи
    #[serde(default)]
    pub badges: Vec<serde_json::Value>,

    /// Начальный шаг веса (для весового товара)
    #[serde(default)]
    pub initial_weight_step: Option<String>,

    /// Минимальный вес (для весового товара)
    #[serde(default)]
    pub min_weight: Option<String>,

    /// Баллы по программе лояльности "Оранжевые очки"
    #[serde(default)]
    pub orange_loyalty_points: Option<u32>,

    /// Доступен ли товар для заказа
    #[serde(default)]
    pub is_available: bool,

    /// Цена за штуку/единицу
    #[serde(default)]
    pub price_piece_unit: Option<serde_json::Value>,
}

impl Into<ProductInfo> for Product {
    fn into(self) -> ProductInfo {
        let price = self.prices.regular.parse::<f64>().unwrap_or_default();
        return ProductInfo {
            id: self.plu.to_string(),
            name: self.name,
            price: price,
            card_price: if let Some(discount) = self.prices.discount {
                discount.parse::<f64>().unwrap_or(price)
            } else {
                price
            },
            rating: self.rating.as_ref().and_then(|v| Some(v.rating_average)),
            rates_count: self.rating.and_then(|v| Some(v.rates_count)), 
            image: self.image_links.normal.get(0).cloned(),
            property: self.property_clarification,
        };
    }
}

/// Ссылки на изображения
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ImageLinks {
    /// Маленькие изображения
    pub small: Vec<String>,

    /// Обычные/большие изображения
    pub normal: Vec<String>,
}

/// Рейтинг товара
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Rating {
    /// Средняя оценка
    pub rating_average: f64,

    /// Количество оценок
    pub rates_count: u32,
}

/// Структура цен
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Prices {
    /// Обычная цена
    #[serde(default)]
    pub regular: String,

    /// Цена со скидкой
    #[serde(default)]
    pub discount: Option<String>,

    /// Цена по специальной акции
    #[serde(default)]
    pub cpd_promo_price: Option<serde_json::Value>,
}

/// Метка скидки или акции
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Label {
    /// Текст метки, например "-12%"
    #[serde(default)]
    pub label: String,

    /// Цвет фона метки
    #[serde(default)]
    pub bg_color: String,

    /// Цвет текста метки
    #[serde(default)]
    pub text_color: String,
}
