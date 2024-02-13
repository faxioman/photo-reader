mod utils;

use std::io::{BufReader, Cursor};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, wasm_bindgen};
use wasm_bindgen::JsCast;
use image::{ImageBuffer, imageops, ImageOutputFormat};
use exif::{In, Tag};
use js_sys::{Promise, Uint8Array};
use web_sys::BlobPropertyBag;
use crate::utils::set_panic_hook;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub async fn get_thumbnail(file : web_sys::File) -> Result<web_sys::Blob, JsValue> {
    set_panic_hook();

    let file_reader: web_sys::FileReader = web_sys::FileReader::new()?;
    let file_loaded_promise = JsFuture::from(Promise::new(&mut |resolve, reject| {
        let fr_c = file_reader.clone();
        let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            if fr_c.result().is_ok() {
                resolve.call1(&JsValue::NULL, &fr_c.result().unwrap()).unwrap();
            } else {
                reject.call0(&JsValue::NULL).unwrap();
            }
        }) as Box<dyn Fn(web_sys::ProgressEvent)>);
        file_reader.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
        onloadend_cb.forget();
    }));

    file_reader.read_as_array_buffer(&file)?;
    let file_content = file_loaded_promise.await?;
    let file_array = js_sys::Uint8Array::new(&file_content);
    let image_vec = file_array.to_vec();

    let img = image::load_from_memory(image_vec.as_slice()).unwrap();
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut BufReader::new(Cursor::new(image_vec)));

    let mut orientation = 1;
    if exif.is_ok() {
        if let Some(field) = exif.unwrap().get_field(Tag::Orientation, In::PRIMARY) {
            orientation = field.value.get_uint(0).unwrap_or(1);
        }
    }

    let corrected_img = match orientation {
        2 => imageops::flip_horizontal(&img),
        3 => imageops::rotate180(&img),
        4 => imageops::flip_horizontal(&imageops::rotate180(&img)),
        5 => imageops::flip_horizontal(&imageops::rotate90(&img)),
        6 => imageops::rotate90(&img),
        7 => imageops::flip_horizontal(&imageops::rotate270(&img)),
        8 => imageops::rotate270(&img),
        _ => ImageBuffer::from(img),
    };


    let (width, height) = corrected_img.dimensions();
    let scale = 320. / width.min(height) as f64;
    let new_width = (width as f64 * scale) as u32;
    let new_height = (height as f64 * scale) as u32;
    let x_offset = (new_width - 320) / 2;
    let y_offset = (new_height - 320) / 2;

    let mut resized_img = imageops::resize(&corrected_img, new_width, new_height, imageops::FilterType::Nearest);
    let cropped_img = imageops::crop(&mut resized_img, x_offset, y_offset, 320, 320);

    let mut buf = Cursor::new(Vec::new());
    cropped_img.to_image().write_to(&mut buf, ImageOutputFormat::WebP).unwrap();

    let uint8_data = Uint8Array::from(buf.into_inner().as_slice());
    let array = js_sys::Array::new();
    array.push(&uint8_data.buffer());

    Ok(web_sys::Blob::new_with_u8_array_sequence_and_options(
        &array,
        BlobPropertyBag::new().type_("image/webp")
    ).unwrap())
}