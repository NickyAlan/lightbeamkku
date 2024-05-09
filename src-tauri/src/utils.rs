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

pub fn find_center_line(arr: U8Array) -> (i32, i32, i32, i32) {
    // find a (horizontal) center line
    // Return: (x1, y1, x2, y2)
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let hp = (0.15 * h as f32) as i32;
    let wp = (0.05 * w as f32) as i32;
    
    let crop = [
        (hp),
        (h as i32 - hp),
        (wp),
        (wp*2)
    ];

    // left point
    let focus_l = arr.slice(s![
        crop[0]..crop[1],crop[2]..crop[3]
    ]).to_owned();
    // add hp (not start at 0)
    let y1 = find_common_value(focus_l, 0) + hp;
    // right point
    let focus_r = arr.slice(s![
        crop[0]..crop[1], w as i32 -crop[3]..w as i32 -crop[2]
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + hp;
    (0, y1, w as i32, y2)
}

/// find center mis-align theta from y1, y2
/// 
/// Return: theta in radius
pub fn find_theta(w: i32, y1: i32, y2: i32) -> f64 {    
    let a = y2 - y1;
    let theta_r = (a as f64 / w as f64).atan();
    -theta_r as f64 // negative for CW rotate
}

/// rotate array CW by theta in radius 
pub fn rotate_array(theta_r: f64, array: U8Array) -> U8Array {
    let h = array.nrows();
    let w = array.ncols();
    let mut rotated = ndarray::Array::zeros((h as usize, w as usize));
    let center_x = w as f64 / 2.;
    let center_y = h as f64 / 2.;   
    
    for i in 0..h {
        for j in 0..w {            
            let x = j as f64 - center_x;
            let y = i as f64 - center_y;
            
            let new_x = x * theta_r.cos() - y * theta_r.sin() + center_x;
            let new_y = x * theta_r.sin() + y * theta_r.cos() + center_y;

            let new_i = new_y.round() as usize;
            let new_j = new_x.round() as usize;
            
            if new_i < h && new_j < w {
                rotated[(new_i, new_j)] = array[(i, j)];
            }
        }
    }
    rotated
}

/// find horizontal lines[y-axis]
/// 
/// Returns: (top(y_avg), center(y_avg), bottom(y_avg)) points
pub fn fint_horizontal_line(arr: U8Array) -> Vec<i32> {
    let mut ypoints = vec![];
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let hp = (0.27 * h as f32) as i32;
    let wp = (0.07 * w as f32) as i32;

    // top line
    // 5 is experimental padding number
    let crop = [5, hp, wp*2, wp*3];
    let focus_l = arr.slice(s![
        crop[0]..crop[1], crop[2]..crop[3]
    ]).to_owned();
    let y1 = find_common_value(focus_l, 0) + crop[0];
    let focus_r = arr.slice(s![
        crop[0]..crop[1],w as i32 - crop[3]..w as i32 - crop[2]
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + crop[0];
    ypoints.push((y1 + y2)/2);

    // center line
    let (_, y1, _, y2) = find_center_line(arr.clone());
    ypoints.push((y1 + y2)/2);
    
    // bottom line    
    let crop = [h as i32 - hp, h as i32 - 5, wp*2, wp*3];
    let focus_l = arr.slice(s![
        crop[0]..crop[1],crop[2]..crop[3]
    ]).to_owned();
    // start at  h as i32 - hp (not 0)
    let y1 = find_common_value(focus_l, 0) + crop[0];
    let focus_r = arr.slice(s![
        crop[0]..crop[1],w as i32 - crop[3]..w as i32 - crop[2]
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + crop[0];
    ypoints.push((y1 + y2)/2);

    ypoints
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