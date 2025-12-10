use crate::error::Result;
use chromiumoxide::{Browser, BrowserConfig, Page, browser::HeadlessMode, cdp::browser_protocol::network::{Cookie, CookieParam, TimeSinceEpoch}};
use std::time::Duration;
use tokio_stream::StreamExt;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

const DEFAULT_LAUNCH_ARGS: [&str; 12] = [
    "--no-first-run",
    "--disable-infobars",
    "--disable-notifications",
    "--disable-default-apps",
    "--disable-sync",
    "--no-sandbox",
    "--disable-blink-features=AutomationControlled",
    "--lang=en_US",
    "--disable-translate",
    "--disable-features=TranslateUI",
    "--no-default-browser-check",
    "--disable-session-crashed-bubble",
];

const DEFAULT_WAIT_PAGE_ELEMENT_DURATION: Duration = Duration::from_secs(15);

pub async fn launch_browser(executable: Option<&str>, headless_mode: HeadlessMode) -> Result<Browser> {
    let mut browser_config_builder = BrowserConfig::builder()
        .disable_default_args()
        .viewport(None)
        .headless_mode(headless_mode)
        .args(DEFAULT_LAUNCH_ARGS);

    if let Some(path) = executable {
        browser_config_builder = browser_config_builder.chrome_executable(path);
    }

    let browser_config = browser_config_builder
        .build()
        .expect("failed build browser config");

    let (browser, mut handler) = Browser::launch(browser_config).await?;

    tokio::spawn(async move { while let Some(Ok(_)) = handler.next().await {} });

    Ok(browser)
}

pub async fn close_browser(b: &mut Browser) {
    if b.close().await.is_err() {
        b.kill().await;
        let _ = b.wait().await;
    }
}

pub fn cookie_into_param(c: Cookie) -> CookieParam {
    return CookieParam { 
        name: c.name, 
        value: c.value, 
        url: None, 
        domain: Some(c.domain), 
        path: Some(c.path), 
        secure: Some(c.secure), 
        http_only: Some(c.http_only), 
        same_site: c.same_site, 
        expires: Some(TimeSinceEpoch::new(c.expires)), 
        priority: Some(c.priority), 
        same_party: None, 
        source_scheme: Some(c.source_scheme), 
        source_port: Some(c.source_port), 
        partition_key: c.partition_key,
    }
}

pub async fn cleanup_browser_pages(b: &Browser) -> Result<()> {
    let pages = b.pages().await?;
    let _ = new_empty_page(b).await?;
    for page in pages {
        let _ = page.close().await;
    }

    Ok(())
}

async fn wait_for_element(p: &Page, selector: &str) -> Result<()> {
    const WAIT: Duration = Duration::from_millis(15);
    while !p
        .evaluate(format!("document.querySelector('{selector}') !== null"))
        .await?
        .into_value::<bool>()?
    {
        tokio::time::sleep(WAIT).await;
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct OpenPageParams<'a> {
    pub url: &'a str,
    pub wait: (&'a str, Duration),
}

pub async fn new_empty_page(b: &Browser) -> Result<Page> {
    let page = b.new_page("about:blank").await?;
    page.set_user_agent(DEFAULT_USER_AGENT).await?;

    Ok(page)
}

pub async fn open_page(b: &Browser, params: &OpenPageParams<'_>) -> Result<Page> {
    let page = new_empty_page(b).await?;

    if params.url != "" {
        page.goto(params.url).await?;
        if params.wait.0 != "" {
            let mut wait_duration = params.wait.1;
            if wait_duration == Duration::ZERO {
                wait_duration = DEFAULT_WAIT_PAGE_ELEMENT_DURATION;
            }
            tokio::time::timeout(wait_duration, wait_for_element(&page, params.wait.0)).await??;
        }
    }

    Ok(page)
}
