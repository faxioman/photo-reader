mod utils;

use std::io::{BufReader, Cursor};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use image::{ImageBuffer, imageops, ImageOutputFormat};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use exif::{In, Tag};
use crate::utils::set_panic_hook;


#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn get_thumbnail(file_input : web_sys::HtmlInputElement, callback: js_sys::Function) {
    set_panic_hook();
    let file_list = file_input.files().expect("Failed to get filelist from File Input!");
    let length = file_list.length();

    if length < 1 {
        alert("Please select at least one file.");
        return;
    }

    for i in 0..length {
        let file = file_list.get(i).expect("Failed to get File from filelist!");
        let file_reader: web_sys::FileReader = web_sys::FileReader::new().unwrap();
        let fr_c = file_reader.clone();
        let cb = callback.clone();
        // create onLoadEnd callback
        let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            let image_file = js_sys::Uint8Array::new(&fr_c.result().unwrap());
            let image_vec = image_file.to_vec();

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
            cropped_img.to_image().write_to(&mut buf, ImageOutputFormat::Png).unwrap();

            let base64_string = STANDARD.encode(buf.into_inner());
            let data_url = format!("data:image/png;base64,{}", base64_string);

            cb.call1(&JsValue::NULL, &JsValue::from(data_url)).unwrap();
        }) as Box<dyn Fn(web_sys::ProgressEvent)>);

        file_reader.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
        file_reader.read_as_array_buffer(&file).expect("blob not readable");
        onloadend_cb.forget();
    }
}