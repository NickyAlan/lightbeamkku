use tauri::Manager;
use std::collections::HashMap;
use ndarray_stats::QuantileExt;
use dicom::pixeldata::image::GrayImage;
use dicom::dictionary_std::tags::{self};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::object::{FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};

type DcmObj = dicom::object::FileDicomObject<dicom::object::InMemDicomObject>;
pub type U16Array = ArrayBase<OwnedRepr<u16>, Dim<[usize; 2]>>;
pub type U8Array = ArrayBase<OwnedRepr<u8>, Dim<[usize; 2]>>;
type I32Array = ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>;
type Obj = FileDicomObject<InMemDicomObject>;

pub fn open_dcm_file(file_path: String) -> Option<DcmObj> {
    match open_file(file_path) {
        Ok(obj) => {
            return Some(obj);
        }, 
        Err(_) => {
            return None;
        }
    }
}

pub fn get_detail(obj: &Obj, tags: Tag) -> String {
    match obj.element(tags) {
            Ok(obj) => {
                let res = obj.to_str().unwrap().to_string();
                if res == "".to_string() {
                    return  " - ".to_string();
                } 
                return res;
            }, 
            Err(_) => {
                return " - ".to_string();
            }
        }
    }

pub fn save_to_image(array: U16Array, save_path: String) {
    // save array to image
    let h = array.nrows();
    let w = array.ncols();
    let u8_gray: Vec<u8> = convert_to_u8(array.clone().into_raw_vec(), array.len());
    let img = array_to_image(u8_gray, h as u32, w as u32);
    img.save(save_path).unwrap();
}

pub fn save_to_image_u8(array: U8Array, save_path: String) {
    // save array to image
    let h = array.nrows();
    let w = array.ncols();
    let img = array_to_image(array.clone().into_raw_vec(), h as u32, w as u32);
    img.save(save_path).unwrap();
}

fn array_to_image(pixel_vec: Vec<u8>, h: u32, w: u32) -> GrayImage {
    GrayImage::from_raw(w, h, pixel_vec).unwrap()
}

pub fn convert_to_u8(pixel_vec: Vec<u16>, size: usize) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::with_capacity(size);
    let max_value = *pixel_vec.iter().max().unwrap() as f32;
    for &value in &pixel_vec {
        let u8_val = ((value as f32 / max_value)* 255.) as u8;
        res.push(u8_val);
    }
    res
}


pub fn find_common_value(arr: U8Array, axis: u8) -> i32 {
    // find common value of each column(axis=0), row(axis=1)
    // return most common pixel value
    let values = argmin(arr, axis);
    // hashmap
    let mut counts: HashMap<usize, u16> = HashMap::new();
    for n in &values {
        let count = counts.entry(*n).or_insert(0);
        *count += 1;
    }
    // then find maximun by value(count) but return key
    let mut max_key = None;
    let mut max_val = std::u16::MIN;
    for (k, v) in counts {
        if v > max_val {
            max_key = Some(k);
            max_val = v;
        }
    }
    max_key.unwrap() as i32
}

pub fn argmin(arr: U8Array, axis: u8) -> Vec<usize> {
    // argmin of each column(axis=0), row(axis=1)
    // return position that has minimum pixel value 
    let rows = arr.nrows();
    let cols = arr.ncols();
    let mut argmins = vec![];
    // argmin
    if axis == 0 {
        for c in 0..cols {
            let mut min_val_col = 255;
            let mut argmin_col = 0;
            for r in 0..rows {
                let pixel_value = arr[(r, c)];
                if pixel_value < min_val_col {
                    min_val_col = pixel_value;
                    argmin_col = r;
                }
            }
            argmins.push(argmin_col);
        }
    } else {
        for r in 0..rows {
            let mut min_val_row = 255;
            let mut argmin_row = 0;
            for c in 0..cols {
                let pixel_value = arr[(r, c)];
                if pixel_value < min_val_row {
                    min_val_row = pixel_value;
                    argmin_row = c;
                }
            }
            argmins.push(argmin_row);
        }
    }
    argmins 
}