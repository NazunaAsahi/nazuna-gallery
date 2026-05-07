use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Document, Element};

const BACKEND_URL: &str = "http://localhost:8787";

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    // Elements
    let gallery = document.get_element_by_id("gallery").expect("should have #gallery");
    let stats = document.get_element_by_id("stats").expect("should have #stats");
    let pagination_top = document.get_element_by_id("pagination-top").expect("should have #pagination-top");
    let pagination_bottom = document.get_element_by_id("pagination-bottom").expect("should have #pagination-bottom");

    // Fetch the list of images from the backend
    let mut opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/list", BACKEND_URL);
    let request = Request::new_with_str_and_init(&url, &opts)?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into().expect("response should be a Response");

    if !resp.ok() {
        return Err(JsValue::from_str("Failed to fetch image list"));
    }

    let json = JsFuture::from(resp.json()?).await?;
    let keys: Vec<String> = serde_wasm_bindgen::from_value(json)?;

    // Update Stats
    stats.set_inner_html(&format!("in request {} pictures", keys.len()));

    // Create Pagination (Mock)
    render_pagination(&document, &pagination_top);
    render_pagination(&document, &pagination_bottom);

    for key in keys {
        let card = create_gallery_card(&document, &key)?;
        gallery.append_child(&card)?;
    }

    Ok(())
}

fn render_pagination(document: &Document, container: &Element) {
    let pages = vec!["[0]", "[1]", "[2]", "[3]", "[>]"];
    for (i, p) in pages.iter().enumerate() {
        let span = document.create_element("span").unwrap();
        span.set_inner_html(p);
        if i == 0 {
            span.set_attribute("class", "current").unwrap();
        }
        container.append_child(&span).unwrap();
    }
}

fn create_gallery_card(document: &Document, key: &str) -> Result<Element, JsValue> {
    let item = document.create_element("div")?;
    item.set_attribute("class", "gallery-item")?;

    // Image Container
    let img_container = document.create_element("div")?;
    img_container.set_attribute("class", "image-container")?;
    
    let img = document.create_element("img")?;
    let img_url = format!("{}/image/{}", BACKEND_URL, key);
    img.set_attribute("src", &img_url)?;
    img.set_attribute("alt", key)?;
    img.set_attribute("loading", "lazy")?;
    
    img_container.append_child(&img)?;
    item.append_child(&img_container)?;

    // Info Bar (Filename & Mock Resolution)
    let info_bar = document.create_element("div")?;
    info_bar.set_attribute("class", "info-bar")?;
    
    let name_span = document.create_element("span")?;
    name_span.set_inner_html(key);
    
    let res_span = document.create_element("span")?;
    res_span.set_inner_html("1920x1080"); // Mock resolution
    
    info_bar.append_child(&name_span)?;
    info_bar.append_child(&res_span)?;
    item.append_child(&info_bar)?;

    // Content Section (Tags)
    let content = document.create_element("div")?;
    content.set_attribute("class", "item-content")?;
    
    let tags_section = document.create_element("div")?;
    tags_section.set_attribute("class", "tags-section")?;
    
    // Parent Tag
    let parent_group = document.create_element("div")?;
    parent_group.set_attribute("class", "tag-group")?;
    parent_group.set_inner_html("<span class='tag-label'>Parent tag:</span> <a href='#' class='tag-link'>swimsuit</a>");
    
    // Child Tags
    let child_group = document.create_element("div")?;
    child_group.set_attribute("class", "tag-group")?;
    child_group.set_inner_html("<span class='tag-label'>Child tags:</span> <a href='#' class='tag-link'>show</a>");
    
    tags_section.append_child(&parent_group)?;
    tags_section.append_child(&child_group)?;
    content.append_child(&tags_section)?;
    item.append_child(&content)?;

    Ok(item)
}
