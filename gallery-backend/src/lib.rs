use worker::*;
use std::io::Cursor;
use photon_rs::native::open_image_from_bytes;
use image::ImageFormat;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new();

    router
        .get_async("/list", |_req, ctx| async move {
            let bucket = ctx.env.bucket("BUCKET")?;
            let objects = bucket.list().execute().await?.objects();
            let keys: Vec<String> = objects.into_iter().map(|obj| obj.key()).collect();
            Response::from_json(&keys)
        })
        .get_async("/image/:name", |req, ctx| async move {
            let name = ctx.param("name").ok_or_else(|| worker::Error::from("Missing name"))?;
            let bucket = match ctx.env.bucket("BUCKET") {
                Ok(b) => b,
                Err(e) => return Response::error(format!("Bucket error: {:?}", e), 500),
            };

            // Check if ?raw=true is present to skip conversion
            let url = req.url()?;
            let is_raw = url.query_pairs().any(|(k, v)| k == "raw" && v == "true");

            console_log!("Request: {}, raw mode: {}", name, is_raw);

            let object = match bucket.get(name).execute().await? {
                Some(obj) => obj,
                None => return Response::error("Not Found", 404),
            };
            
            let body_bytes = match object.body() {
                Some(b) => b.bytes().await?,
                None => return Response::error("No Body", 404),
            };

            if is_raw {
                console_log!("Raw mode: Returning original {} bytes", body_bytes.len());
                let mut headers = Headers::new();
                // Try to guess content type from extension or use octet-stream
                if name.ends_with(".png") { headers.set("Content-Type", "image/png")?; }
                else if name.ends_with(".webp") { headers.set("Content-Type", "image/webp")?; }
                else { headers.set("Content-Type", "application/octet-stream")?; }
                headers.set("Access-Control-Allow-Origin", "*")?;
                return Ok(Response::from_bytes(body_bytes)?.with_headers(headers));
            }

            // --- Conversion Logic ---
            console_log!("Converting {} to JPG...", name);
            
            // 1. Decode to photon image
            let photon_img = match open_image_from_bytes(&body_bytes) {
                Ok(i) => i,
                Err(e) => {
                    console_log!("Photon Decode Error: {:?}", e);
                    return Response::error("Decode Error", 500);
                }
            };

            // 2. Convert to DynamicImage (image crate) for more explicit encoding
            let raw_pixels = photon_img.get_raw_pixels();
            let width = photon_img.get_width();
            let height = photon_img.get_height();
            
            let img_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, raw_pixels)
                .ok_or_else(|| worker::Error::from("Failed to create image buffer"))?;
            let dynamic_img = image::DynamicImage::ImageRgba8(img_buffer);

            // 3. Encode to JPG into a clean vector
            let mut jpg_buffer = Vec::new();
            let mut cursor = Cursor::new(&mut jpg_buffer);
            match dynamic_img.write_to(&mut cursor, ImageFormat::Jpeg) {
                Ok(_) => (),
                Err(e) => {
                    console_log!("Image Encode Error: {:?}", e);
                    return Response::error("Encode Error", 500);
                }
            };

            console_log!("Success: Converted to JPG ({} bytes)", jpg_buffer.len());

            let mut headers = Headers::new();
            headers.set("Content-Type", "image/jpeg")?;
            headers.set("Access-Control-Allow-Origin", "*")?;
            headers.set("Cache-Control", "public, max-age=3600")?;

            Ok(Response::from_bytes(jpg_buffer)?.with_headers(headers))
        })
        .get("/", |_req, _ctx| {
            Response::ok("Gallery Backend is running.")
        })
        .run(req, env)
        .await
        .map(|mut res| {
            let headers = res.headers_mut();
            let _ = headers.set("Access-Control-Allow-Origin", "*");
            res
        })
}
