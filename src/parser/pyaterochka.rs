use crate::browser_utils::{self as bu, OpenPageParams};
use crate::db;
use crate::error::Result;
use crate::parser::models::pyaterochka as models;
use chromiumoxide::cdp::browser_protocol::network::Cookie;
use chromiumoxide::{Browser, browser::HeadlessMode};
use rand::seq::{IndexedRandom, SliceRandom};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

pub const MAX_CATALOG_API_LIMIT: u16 = 499;

pub const MAIN_CATALOG_LIST: [Catalog; 17] = [
    Catalog::GotovayaEda,
    Catalog::OvoshchiFruktyOrekhi,
    Catalog::MolochnayaProduktsiyaIYaytso,
    Catalog::KhlebIVypechka,
    Catalog::MyasoPtitsaKolbasy,
    Catalog::RybaIMoreprodukty,
    Catalog::Sladosti,
    Catalog::SnekiIChipsy,
    Catalog::Bakaleya,
    Catalog::ZamorozhennyeProdukty,
    Catalog::VodaINapitki,
    Catalog::ZdorovyyVybor,
    Catalog::DlyaDetey,
    Catalog::DlyaZhivotnykh,
    Catalog::KrasotaGigienaApteka,
    Catalog::StirkaIUborka,
    Catalog::DlyaDomaIDachi,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Catalog {
    GotovayaEda,
    OvoshchiFruktyOrekhi,
    MolochnayaProduktsiyaIYaytso,
    KhlebIVypechka,
    MyasoPtitsaKolbasy,
    RybaIMoreprodukty,
    Sladosti,
    SnekiIChipsy,
    Bakaleya,
    ZamorozhennyeProdukty,
    VodaINapitki,
    ZdorovyyVybor,
    DlyaDetey,
    DlyaZhivotnykh,
    KrasotaGigienaApteka,
    StirkaIUborka,
    DlyaDomaIDachi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CatalogFilter {
    Default,
    PriceDesc,
    PriceAsc,
}

impl CatalogFilter {
    pub fn as_url_query(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::PriceDesc => "&order_by=price_desc",
            Self::PriceAsc => "&order_by=price_asc",
        }
    }
}

const CATALOG_FILTERS_LIST: [CatalogFilter; 3] = [
    CatalogFilter::Default,
    CatalogFilter::PriceDesc,
    CatalogFilter::PriceAsc,
];

impl Catalog {
    pub fn as_catalog_id(&self) -> &'static str {
        match self {
            Catalog::GotovayaEda => "251C12884",
            Catalog::OvoshchiFruktyOrekhi => "251C12886",
            Catalog::MolochnayaProduktsiyaIYaytso => "251C12887",
            Catalog::KhlebIVypechka => "251C12888",
            Catalog::MyasoPtitsaKolbasy => "251C12889",
            Catalog::RybaIMoreprodukty => "251C12890",
            Catalog::Sladosti => "251C12900",
            Catalog::SnekiIChipsy => "251C12901",
            Catalog::Bakaleya => "251C12902",
            Catalog::ZamorozhennyeProdukty => "251C12903",
            Catalog::VodaINapitki => "251C12904",
            Catalog::ZdorovyyVybor => "251C12905",
            Catalog::DlyaDetey => "251C12906",
            Catalog::DlyaZhivotnykh => "251C12907",
            Catalog::KrasotaGigienaApteka => "251C12908",
            Catalog::StirkaIUborka => "251C12909",
            Catalog::DlyaDomaIDachi => "251C12910",
        }
    }

    pub fn as_api_url(&self, store_id: &str, limit: u16) -> String {
        let mut rng = rand::rng();
        let filter = CATALOG_FILTERS_LIST
            .choose(&mut rng)
            .unwrap()
            .as_url_query();
        format!(
            "https://5d.5ka.ru/api/catalog/v2/stores/{store_id}/categories/{catalog_id}/products?mode=delivery&include_restrict=true&limit={limit}{filter}",
            catalog_id = self.as_catalog_id()
        )
    }
}

pub fn store_from_coord_url(lat: f32, lon: f32) -> String {
    format!("https://5d.5ka.ru/api/orders/v1/orders/stores/?lat={lat}&lon={lon}")
}

pub const HOME_PAGE_URL: &str = "https://5ka.ru/";

pub async fn read_pyaterochka_coords(path: Option<&str>) -> Result<Vec<[f32; 2]>> {
    let coords_data =
        tokio::fs::read_to_string(path.unwrap_or("pyaterochka_stores_coord.json")).await?;
    let mut pyaterochka_stores_coord = serde_json::from_str::<Vec<[f32; 2]>>(&coords_data)?;
    let mut rng = rand::rng();
    pyaterochka_stores_coord.shuffle(&mut rng);

    Ok(pyaterochka_stores_coord)
}

async fn set_cookies_from_path(b: &Browser, path: &str) -> Result<()> {
    if !std::fs::exists(path).unwrap_or(false) {
        return Ok(());
    }
    let cookies_json = tokio::fs::read_to_string(path).await?;
    let cookies_param = serde_json::from_str::<Vec<Cookie>>(&cookies_json)?
        .into_iter()
        .map(bu::cookie_into_param)
        .collect::<Vec<_>>();
    if !cookies_param.is_empty() {
        b.set_cookies(cookies_param).await?;
    }
    Ok(())
}

async fn pyaterochka_update_cookies_with_borwser(
    b: &Browser,
    cookies_store_path: Option<&str>,
) -> Result<Vec<Cookie>> {
    let page = bu::open_page(
        &b,
        &bu::OpenPageParams {
            url: HOME_PAGE_URL,
            ..Default::default()
        },
    )
    .await?;

    tokio::time::sleep(Duration::from_secs(5)).await;

    while let Some(url) = page.url().await? {
        if url.as_str() == HOME_PAGE_URL {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let cookies = b.get_cookies().await?;
    let cookies_json = serde_json::ser::to_string_pretty(&cookies)?;
    tokio::fs::write(
        cookies_store_path.unwrap_or("pyaterochka_cookies"),
        cookies_json,
    )
    .await?;

    let _ = page.close().await;

    Ok(cookies)
}

async fn pyaterochka_update_cookies(
    executable: Option<&str>,
    cookies_store_path: Option<&str>,
) -> Result<Vec<Cookie>> {
    let mut b = bu::launch_browser(executable, HeadlessMode::False).await?;

    if let Some(path) = cookies_store_path {
        set_cookies_from_path(&b, path).await?;
    }

    let cookies = pyaterochka_update_cookies_with_borwser(&b, cookies_store_path).await?;

    bu::close_browser(&mut b).await;

    Ok(cookies)
}

#[derive(Debug, Default)]
pub struct ParseConfig<'a> {
    pub browser_executable: Option<&'a str>,
    pub cookies_store_path: Option<&'a str>,
    pub pyaterochka_stores_coord_path: Option<&'a str>,
    pub sleep_millis_for_each_catalog: Option<u64>,
}

pub async fn start_parsing<'a>(pc: &ParseConfig<'a>) -> Result<()> {
    pyaterochka_update_cookies(pc.browser_executable, pc.cookies_store_path).await?;
    let b = Arc::new(bu::launch_browser(pc.browser_executable, HeadlessMode::True).await?);
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    {
        let b = b.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
            println!("\nCtrl+C received, initiating graceful shutdown...");
            let browser_ref = unsafe { &mut *(Arc::<Browser>::as_ptr(&b) as *mut Browser) };
            bu::close_browser(browser_ref).await;
            let _ = tx.send(());
        });
    }
    if let Some(cookies_store_path) = pc.cookies_store_path {
        set_cookies_from_path(&b, cookies_store_path).await?;
    }
    let stores_coords = read_pyaterochka_coords(pc.pyaterochka_stores_coord_path).await?;
    let store_by_coord_urls = stores_coords
        .into_iter()
        .map(|v| store_from_coord_url(v[0], v[1]))
        .collect::<Vec<_>>();
    loop {
        if rx.try_recv().is_ok() {
            return Ok(());
        }
        let mut stores_set = HashSet::new();
        for (sn, s) in store_by_coord_urls.iter().enumerate() {
            let _ = bu::cleanup_browser_pages(&b).await;
            let page = bu::open_page(
                &b,
                &OpenPageParams {
                    url: s,
                    wait: ("pre", Duration::from_secs(5)),
                },
            )
            .await;
            if page.is_err() {
                eprintln!("Not found store info content block");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            let page = unsafe { page.unwrap_unchecked() };

            let find_element = page.find_element("pre").await;
            let content = find_element
                .unwrap()
                .inner_text()
                .await?
                .unwrap_or_default();
            let store_api_info = serde_json::from_str::<models::StoreApiInfo>(&content);
            if store_api_info.is_err() {
                eprintln!("Not found store info content");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            let store_api_info = unsafe { store_api_info.unwrap_unchecked() };
            let store_info = Arc::new(Into::<models::StoreInfo>::into(store_api_info));
            let _ = page.close().await;
            if !stores_set.insert(store_info.id.clone()) {
                continue;
            }
            println!(
                "---------------------------------------\n{sn}. {} - {}\n---------------------------------------",
                store_info.address, store_info.city
            );
            let mut join_set = JoinSet::new();
            for (cn, c) in MAIN_CATALOG_LIST.iter().enumerate() {
                {
                    let b = b.clone();
                    let store_info = store_info.clone();
                    join_set.spawn(async move {
                        let url = c.as_api_url(&store_info.id, MAX_CATALOG_API_LIMIT);
                        let page = bu::open_page(
                            &b,
                            &bu::OpenPageParams {
                                url: url.as_str(),
                                wait: ("pre", Duration::from_secs(9)),
                            },
                        )
                        .await?;
                        let find_element = page.find_element("pre").await?;
                        let content = find_element.inner_text().await?.unwrap_or_default();
                        let catalog = serde_json::from_str::<models::Catalog>(&content)?;
                        let result = models::CatalogInfoWithTime::from_catalog_with_id(
                            catalog,
                            c.as_catalog_id().into(),
                            None,
                        );
                        let _ = page.close().await;
                        println!("{cn}. {:?} {}", c, result.info.products.len());
                        Result::Ok(result)
                    });
                }
                tokio::time::sleep(
                    Duration::from_millis(pc.sleep_millis_for_each_catalog.unwrap_or(700))
                ).await;
            }
            let catalogs = join_set
                .join_all()
                .await
                .into_iter()
                .inspect(|r| {
                    if r.is_err() {
                        eprintln!("Some error while parse catalog page");
                    }
                })
                .filter(Result::is_ok)
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            db::pyaterochka_insert_data(&store_info, &catalogs)?;
        }
    }
}
