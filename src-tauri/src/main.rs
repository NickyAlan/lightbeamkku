// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod utils;
use crate::utils::{open_dcm_file, save_to_image, get_detail, convert_to_u8, save_to_image_u8, find_common_value, find_center_line, find_theta, rotate_array, fint_horizontal_line};
use crate::utils::{U8Array, U16Array};
use std::collections::HashMap;
use ndarray_stats::QuantileExt;
use dicom::pixeldata::image::GrayImage;
use dicom::dictionary_std::tags::{self, FOCAL_DISTANCE, SPHERE_POWER};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::object::{FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};

// TYPE
type DcmObj = dicom::object::FileDicomObject<dicom::object::InMemDicomObject>;
type I32Array = ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>;
type Obj = FileDicomObject<InMemDicomObject>;


#[tauri::command]
fn processing(file_path: String, save_path: String) {
    match open_dcm_file(file_path) {
        Some(obj) => {
            let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
            let arr = pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();

            // details
            let hospital = get_detail(&obj, tags::INSTITUTION_NAME);
            // ...
            let new_arr = arr_correction(arr);
            let (x1, y1, x2, y2) = find_center_line(new_arr.clone());
            let theta_r = find_theta(x2, y1, y2);
            let arr = rotate_array(theta_r, new_arr);
            let [topy, midy, boty] = fint_horizontal_line(arr).as_slice().try_into().unwrap();
            dbg!(topy, midy, boty);
        }, 
        None => {
            
        }
    }
}

fn arr_correction(arr: U16Array) -> U8Array {
    // crop array as expect.
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    // convert arr to vec to convert pixel value [0, 255]
    let u8_gray: Vec<u8> = convert_to_u8(arr.clone().into_raw_vec(), arr.len());
    let mut new_arr = Array::from_shape_vec((h, w), u8_gray).unwrap();
    let shape = new_arr.shape();
    let h = shape[0];
    let w = shape[1];
    // crop only area of test tool
    let p = 0.24; // experimental number
    if (h*w) > (2000*2000) {
        let crop = [
            (p*(h as f32)) as i32,
            (h as f32 * (1.0-p)) as i32,
            (w as f32 * p) as i32,
            (w as f32 * (1.0-p)) as i32 
        ];
        new_arr = new_arr.slice(
            s![crop[0]..crop[1], crop[2]..crop[3]]
        ).to_owned();
    }
    new_arr
}



fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![processing])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
