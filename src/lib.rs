use image::Rgba;
use imageproc::drawing::{draw_text_mut, Canvas};
use punycode::decode;
use rusttype::{Font, Scale};
use worker::*;

mod r2;
mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    let router = Router::new();
    router
        // return subdomain html
        .get("/", |req, _| {
            let host = req.headers().get("host").unwrap_or_default();
            match host {
                Some(host) => {
                    let (subdomain, host) = parse_host(host);
                    let subdomain_punycode = convert_punycode(subdomain);
                    let mut title = format!("{}おわりや", subdomain_punycode);
                    let mut message = format!("{}おわりが売ってる", subdomain_punycode);

                    match subdomain_punycode.as_str() {
                        "jinsei" => {
                            title = "人生おわりや".to_string();
                            message = "もうだめ".to_string();
                        }
                        "konnendomo" => {
                            title = "今年度もおわりや".to_string();
                            message = "おめでとうございます".to_string();
                        }
                        "kotoshimo" => {
                            title = "今年もおわりや".to_string();
                            message = "あけましておめでとうございます".to_string();
                        }
                        "kyoumo" => {
                            title = "今日もおわりや".to_string();
                            message = "一日お疲れ様でした".to_string();
                        }
                        "" => {
                            title = "おわりや".to_string();
                            message = "おわりが売ってる".to_string();
                        }
                        _ => {}
                    }
                    let html = create_html(title, message, host);
                    Response::from_html(html)
                }
                None => Response::ok(""),
            }
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .get_async("/favicon.ico", |req, ctx| async move {
            let font = match r2::get(ctx, "Koruri-Extrabold.ttf").await {
                Some(font_bytes) => Font::try_from_vec(font_bytes).unwrap(),
                None => return Response::error("Internal server error: cant find font", 500),
            };

            let host = req.headers().get("host").unwrap_or_default();
            let subdomain_punycode = match host {
                Some(host) => {
                    let (subdomain, _) = parse_host(host);
                    convert_punycode(subdomain)
                }
                None => "".to_string(),
            };
            let emoji = owariya_image(subdomain_punycode, font);
            let emoji_png = match write_image(emoji, image::ImageOutputFormat::Ico) {
                Some(emoji_png) => emoji_png,
                None => return Response::error("Internal server error: cant create image", 500),
            };
            Response::from_bytes(emoji_png)
        })
        .get_async("/owariya.png", |req, ctx| async move {
            let font = match r2::get(ctx, "Koruri-Extrabold.ttf").await {
                Some(font_bytes) => Font::try_from_vec(font_bytes).unwrap(),
                None => return Response::error("Internal server error: cant find font", 500),
            };

            let host = req.headers().get("host").unwrap_or_default();
            let subdomain_punycode = match host {
                Some(host) => {
                    let (subdomain, _) = parse_host(host);
                    convert_punycode(subdomain)
                }
                None => "".to_string(),
            };
            let emoji = owariya_image(subdomain_punycode, font);
            let emoji_png = match write_image(emoji, image::ImageOutputFormat::Png) {
                Some(emoji_png) => emoji_png,
                None => return Response::error("Internal server error: cant create image", 500),
            };
            Response::from_bytes(emoji_png)
        })
        .run(req, env)
        .await
}

fn write_image(dynamic: image::DynamicImage, format: image::ImageOutputFormat) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    dynamic.write_to(&mut buf, format).ok()?;
    Some(buf)
}

fn owariya_image(subdomain: String, font: Font) -> image::DynamicImage {
    let height = 256;
    let width = 256;
    let background_color = Rgba([192u8, 192u8, 192u8, 255u8]);
    let font_color = Rgba([0u8, 0u8, 0u8, 255u8]);

    let mut img = image::DynamicImage::new_rgb8(width, height);

    let x = 0;
    let mut y = 0;
    let height_f32 = height as f32;
    let width_f32 = width as f32;

    // fill background gray
    for x in 0..width {
        for y in 0..height {
            img.draw_pixel(x, y, background_color)
        }
    }

    if subdomain.is_empty() {
        let owa = "おわ";
        let riya = "りや";
        let scale_owa = get_scale_by_font(height_f32 / 2.0, width_f32, &font, owa);
        let scale_riya = get_scale_by_font(height_f32 / 2.0, width_f32, &font, riya);
        draw_text_mut(&mut img, font_color, x, y, scale_owa, &font, owa);
        y += height / 2;
        draw_text_mut(&mut img, font_color, x, y, scale_riya, &font, riya);
    } else {
        let owariya = "おわりや";
        let scale_subdomain = get_scale_by_font(height_f32 / 2.0, width_f32, &font, &subdomain);
        let scale_owariya = get_scale_by_font(height_f32 / 2.0, width_f32, &font, owariya);
        draw_text_mut(
            &mut img,
            font_color,
            x,
            y,
            scale_subdomain,
            &font,
            &subdomain,
        );
        y += height / 2;
        draw_text_mut(&mut img, font_color, x, y, scale_owariya, &font, owariya);
    }

    img
}

fn get_scale_by_font(height: f32, width: f32, font: &Font, text: &str) -> Scale {
    let mut glyph_width_sum = 0.0;
    for c in text.chars() {
        let glyph = font.glyph(c).scaled(Scale::uniform(height));
        glyph_width_sum += glyph.h_metrics().advance_width;
    }
    if glyph_width_sum == 0.0 {
        glyph_width_sum = 1.0;
    }
    Scale {
        x: height * width / glyph_width_sum,
        y: height,
    }
}

fn parse_host(host: String) -> (String, String) {
    // if owari.shop subdomain will be empty
    let mut subdomain = String::new();
    let domain = host;
    if domain.contains(".owari.shop") {
        subdomain = domain.replace(".owari.shop", "");
    }
    (subdomain, domain)
}

fn convert_punycode(sub: String) -> String {
    let mut subdomain = sub;
    if subdomain.contains("xn--") {
        subdomain = subdomain.replace("xn--", "");
        subdomain = decode(&subdomain).unwrap_or_default();
    }
    subdomain
}

fn create_html(title: String, message: String, domain: String) -> String {
    let html = include_str!("../static/index.html.tmpl");
    html.replace("{{ .Title }}", &title)
        .replace("{{ .Message }}", &message)
        .replace("{{ .Domain }}", &domain)
}
