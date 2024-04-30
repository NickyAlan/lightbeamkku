// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod utils;
use crate::utils::{open_dcm_file, save_to_image, get_detail};
use tauri::Manager;
use std::collections::HashMap;
use ndarray_stats::QuantileExt;
use dicom::pixeldata::image::GrayImage;
use dicom::dictionary_std::tags::{self};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::object::{FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};

// TYPE
type DcmObj = dicom::object::FileDicomObject<dicom::object::InMemDicomObject>;
type U16Array = ArrayBase<OwnedRepr<u16>, Dim<[usize; 2]>>;
type I32Array = ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>;
type Obj = FileDicomObject<InMemDicomObject>;


#[tauri::command]
fn processing(file_path: String, save_path: String) {
    match open_dcm_file(file_path) {
        Some(obj) => {
            let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
            let arr=  pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();

            // details
            let hospital = get_detail(&obj, tags::INSTITUTION_NAME);
            dbg!(hospital);
            save_to_image(arr, save_path);
        }, 
        None => {
            
        }
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![processing])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
