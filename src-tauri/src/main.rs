// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod utils;
use crate::utils::{open_dcm_file, save_to_image, get_detail, convert_to_u8, save_to_image_u8,find_common_value, find_center_line, find_theta, rotate_array, fint_horizontal_line, find_vertical_line, boxs_posision, find_edges_pos, get_result};
use crate::utils::{U8Array, U16Array, pixel2cm, cm2pixel, U128Array};
use std::collections::HashMap;
use ndarray_stats::QuantileExt;
use dicom::pixeldata::image::{flat, GrayImage};
use dicom::dictionary_std::tags::{self, FOCAL_DISTANCE, NUMBER_OF_COMPENSATORS, SPHERE_POWER};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::object::{pixeldata, FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};
use utils::get_crop_area;

// TYPE
type DcmObj = dicom::object::FileDicomObject<dicom::object::InMemDicomObject>;
type I32Array = ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>;
type Obj = FileDicomObject<InMemDicomObject>;

#[tauri::command]
fn preview(file_path: String, save_path: String) {
    match open_dcm_file(file_path) {
        Some(obj) => {      
            let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
            let arr = pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();
            save_to_image(arr, save_path);
        },
        None => {

        }
    } 
}

#[tauri::command]
fn processing(file_paths: Vec<String>, save_path: String) {
    dbg!(&file_paths, &save_path);
    let large_field = file_paths[0].to_owned();
    let small_field = file_paths[1].to_owned();
    // let mut arrays = vec![];
    match open_dcm_file(large_field) {
        Some(obj) => {
            // Large field: find pattern of test-tool 
            let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
            let arr = pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();
            // save_to_image(arr.clone(), save_path);
            
            // Detector details
            let hospital = get_detail(&obj, tags::INSTITUTION_NAME);
            // ...

            // Find Test-Tool
            let arr = arr_correction(arr);
            // Find Center Line
            let (_, _, _, _, theta_r) = find_center_line(arr.clone());
            // Adjust angle
            let rotated_arr = rotate_array(theta_r, arr.clone());
            // Fine Lines in Rotated array
            let ypoints = fint_horizontal_line(rotated_arr.clone());
            let xpoints = find_vertical_line(rotated_arr);

            // Small field
            match open_dcm_file(small_field) {
                Some(obj) => {
                    let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
                    let arr = pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();
                    let arr = arr_correction(arr);
                    let rotated_arr2 = rotate_array(theta_r, arr);
                    
                    // Find the Edges
                    // boxss_position(area for crop)
                    let boxs_pos = boxs_posision(&xpoints, &ypoints, rotated_arr2.clone());
                    // get crop area
                    let crop_areas = get_crop_area(boxs_pos.clone(), rotated_arr2);
                    // edges positions
                    let edges_pos = find_edges_pos(crop_areas, boxs_pos);
                    // Result: left, right, top, bottom [x1, y1, x2, y2, length]
                    let (center_p, res_xy, res_length, res_err) = get_result(edges_pos, &xpoints, &ypoints);

                    // let add_arr = add_arrays(arrays[0].clone(), arr2);
                    // save_to_image_u8(add_arr, "c:/Users/alant/Desktop/added.jpg".to_string());
                },
                None => {

                }
            }
        }, 
        None => {
            
        }
    }
}

fn arr_correction(mut arr: U16Array) -> U16Array {
    // crop array as expect.
    // Find Test-Tool
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    
    // Not to u8 (for more precision U16)
    // convert arr to vec to convert pixel value [0, 255]
    // let u8_gray: Vec<u8> = convert_to_u8(arr.clone().into_raw_vec(), arr.len());
    // let mut new_arr = Array::from_shape_vec((h, w), u8_gray).unwrap();
    // let shape = new_arr.shape();
    // let h = shape[0];
    // let w = shape[1];

    // crop only area of test tool
    let p = 0.24; 
    // if it larger than 2000^2, it not crop yet
    if (h*w) > (2000*2000) { 
        let crop = [
            (p*(h as f32)) as i32,
            (h as f32 * (1.0-p)) as i32,
            (w as f32 * p) as i32,
            (w as f32 * (1.0-p)) as i32 
        ];
        arr = arr.slice(
            s![crop[0]..crop[1], crop[2]..crop[3]]
        ).to_owned();
    }

    arr
}

/// Return
/// left, right, top, bottom
/// 
/// x1, x2, y1, y2, length, err
fn find_rentangular_details(arr: U8Array, xpoints: Vec<i32>, ypoints: Vec<i32>) -> Vec<[f32; 6]> {
    let shape = arr.shape();
    let h = shape[0];
    // res: [x1, x2, y1, y2, length, err]
    let mut res = vec![];
    // left edge
    // estimate line between top and center: to escepe center black line
    let center_p = [xpoints[1], (ypoints[0] + ypoints[1])/2];
    let left_p = [xpoints[0], (ypoints[0] + ypoints[1])/2];
    let crop_ratio = 0.2;
    let add_horiontal_crop = 
        (left_p[0] as f32 *(1.0-crop_ratio)) as i32 
        + (center_p[0] as f32 * crop_ratio) as i32
        - left_p[0];
    let add_vertical_crop = (h as f32 * 0.04) as i32;
    let top_left_p = [
        (left_p[0] - add_horiontal_crop).max(0), // prevent negative
        left_p[1] - add_vertical_crop
    ];
    let bottom_right_p = [
        left_p[0] + add_horiontal_crop,
        left_p[1] + add_vertical_crop
    ];
    // crop area
    let left_focus = arr.slice(s![
        top_left_p[1]..bottom_right_p[1],
        top_left_p[0]..bottom_right_p[0]
    ]).to_owned();
    let fw_idx = first_white(left_focus.clone(), true, "left");
    let left_find_black = left_focus.slice(s![
        0..left_focus.nrows(), fw_idx..left_focus.ncols()
    ]).to_owned();
    let left_find_black = left_find_black.mapv(|x| x as u128); // overflow
    let cut_off = left_find_black.mean().unwrap();
    let binary_arr = to_binary_arr(left_find_black, cut_off);
    let lw_idx = first_white(binary_arr, true, "left") + fw_idx;
    let left_bte_idx = ((fw_idx + lw_idx)/2) as i32 + top_left_p[0]; // to reset position same as large arr
    let pixel_diff = xpoints[1] - left_bte_idx;
    let length_x1 = pixel2cm(&ypoints, pixel_diff, false);
    let length_diff = length_x1 - 9.0;
    res.push([
        left_bte_idx as f32, left_bte_idx as f32, ypoints[0] as f32, ypoints[2] as f32, length_x1, length_diff
    ]);
    
    // right edge
    let right_p = [xpoints[2], (ypoints[0] + ypoints[1])/2];
    let top_left_p = [
        right_p[0] - add_horiontal_crop,
        right_p[1] - add_vertical_crop
    ];
    let bottom_right_p = [
        right_p[0] + add_horiontal_crop,
        right_p[1] + add_vertical_crop
    ];
    // crop area
    let right_focus = arr.slice(s![
        top_left_p[1]..bottom_right_p[1],
        top_left_p[0]..bottom_right_p[0]
    ]).to_owned();
    let fw_idx = first_white(right_focus.clone(), true, "right");
    let right_find_black = right_focus.slice(s![
        0..right_focus.nrows(), 0..fw_idx
    ]).to_owned();
    let right_find_black = right_find_black.mapv(|x| x as u128); // overflow
    let cut_off = right_find_black.mean().unwrap();
    let binary_arr = to_binary_arr(right_find_black, cut_off);
    let lw_idx = first_white(binary_arr, true, "right");
    let right_bte_idx = ((fw_idx + lw_idx)/2) as i32 + top_left_p[0]; // to reset position same as large arr
    let pixel_diff = right_bte_idx - xpoints[1];
    let length_x2 = pixel2cm(&ypoints, pixel_diff, false);
    let length_diff = length_x2 - 9.0;
    res.push([
        right_bte_idx as f32, right_bte_idx as f32, ypoints[0] as f32, ypoints[2] as f32, length_x2, length_diff
    ]);

    // top edge
    let top_p = [(xpoints[1] + xpoints[2])/2, ypoints[0]];
    let top_left_p = [
        top_p[0] - add_vertical_crop,
        (top_p[1] - add_horiontal_crop).max(0)
    ];
    let bottom_right_p = [
        top_p[0] + add_vertical_crop,
        top_p[1] + add_horiontal_crop
    ];
    // crop area
    let top_focus = arr.slice(s![
        top_left_p[1]..bottom_right_p[1],
        top_left_p[0]..bottom_right_p[0]
    ]).to_owned();
    let fw_idx = first_white(top_focus.clone(), false, "top");
    let top_find_black = top_focus.slice(s![
        fw_idx..top_focus.nrows(), 0..top_focus.ncols()
    ]).to_owned();
    let top_find_black: ArrayBase<OwnedRepr<u128>, Dim<[usize; 2]>> = top_find_black.mapv(|x| x as u128); // overflow
    let cut_off = top_find_black.mean().unwrap();
    let binary_arr = to_binary_arr(top_find_black, cut_off);
    let lw_idx = first_white(binary_arr, false, "top") + fw_idx;
    let top_bte_idx = ((fw_idx + lw_idx)/2) as i32 + top_left_p[1]; // to reset position same as large arr
    let pixel_diff = ypoints[1] - top_bte_idx;
    let length_y1 = pixel2cm(&ypoints, pixel_diff, false);
    let length_diff = length_y1 - 7.0;
    res.push([
        xpoints[0] as f32, xpoints[2] as f32, top_bte_idx as f32, top_bte_idx as f32, length_y1, length_diff
    ]);
    
    // bottom edge
    let bottom_p = [(xpoints[1] + xpoints[2])/2, ypoints[2]];
    let top_left_p = [
        bottom_p[0] - add_vertical_crop,
        bottom_p[1] - add_horiontal_crop
    ];
    let bottom_right_p = [
        bottom_p[0] + add_vertical_crop,
        bottom_p[1] + add_horiontal_crop
    ];
    // crop area
    let bottom_focus = arr.slice(s![
        top_left_p[1]..bottom_right_p[1],
        top_left_p[0]..bottom_right_p[0],
    ]).to_owned();
    let fw_idx = first_white(bottom_focus.clone(), false, "bottom");
    let bottom_find_black = bottom_focus.slice(s![
        0..fw_idx, 0..bottom_focus.ncols()
    ]).to_owned();      
    let bottom_find_black: ArrayBase<OwnedRepr<u128>, Dim<[usize; 2]>> = bottom_find_black.mapv(|x| x as u128); // overflow
    let cut_off = bottom_find_black.mean().unwrap();
    let binary_arr = to_binary_arr(bottom_find_black, cut_off);
    let lw_idx = first_white(binary_arr, false, "bottom");
    let bottom_bte_idx = ((fw_idx + lw_idx)/2) as i32 + top_left_p[1]; // to reset position same as large arr
    let pixel_diff = bottom_bte_idx - ypoints[1];
    let length_y2 = pixel2cm(&ypoints, pixel_diff, false);
    let length_diff = length_y2 - 7.0;
    res.push([
        xpoints[0] as f32, xpoints[2] as f32, bottom_bte_idx as f32, bottom_bte_idx as f32, length_y2, length_diff
    ]);
    res
}

/// add 2 array
fn add_arrays(arr1: U8Array, arr2: U8Array) -> U8Array {
    let nrows = arr1.nrows();
    let ncols = arr1.ncols();
    let max_v = 510.0; // 255*2
    let mut add_arr = vec![];
    for r in 0..nrows {
        for c in 0..ncols {
            let add_v = arr1[(r, c)] as u16 + arr2[(r, c)] as u16;
            let v_u8 = ((add_v as f32/max_v) * 255.0) as u8;
            add_arr.push(v_u8);
        }
    }
    Array::from_shape_vec((nrows, ncols), add_arr).unwrap()
}

/// find idx that first white then -1 for actually area
fn first_white(arr: U8Array, by_col: bool, position: &str) -> usize {
    let shape = arr.shape();
    let nrows = shape[0];
    let ncols = shape[1];
    let white_pixel_val = *arr.max().unwrap();
    let mut first_val_idx = vec![];
    let black_pos = ["right", "bottom"];
    let is_black = black_pos.contains(&position);
    if is_black {
        // right and bottom : start from last col and row
        if by_col {
            for row in 0..nrows {
                for col in 0..ncols {
                    let pixel_val = arr[(nrows-1-row, ncols-1-col)];
                    if pixel_val < white_pixel_val {
                        first_val_idx.push(ncols-col);
                        break;
                    }
                }
            }
        } else {
            for col in 0..ncols {
                for row in 0..nrows {
                    let pixel_val = arr[(nrows-1-row, ncols-1-col)];
                    if pixel_val < white_pixel_val {
                        first_val_idx.push(nrows-row);
                        break;
                    }
                }
            }
        }
    } else {
        // left and top
        if by_col {
            for row in 0..nrows {
                for col in 0..ncols {
                    let pixel_val = arr[(row, col)];
                    if pixel_val < white_pixel_val {
                        first_val_idx.push(col-1);
                        break;
                    }
                }
            }
        } else {
            for col in 0..ncols {
                for row in 0..nrows {
                    let pixel_val = arr[(row, col)];
                    if pixel_val < white_pixel_val {
                        first_val_idx.push(row-1);
                        break;
                    }
                }
            }
        }
    }
    let idx = most_common_val(first_val_idx);
    idx
}

fn most_common_val(values: Vec<usize>) -> usize {
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
    max_key.unwrap()
}

fn to_binary_arr(arr: U128Array, cut_off: u128) -> U8Array {
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let binary_arr = arr.iter()
        .map(|&x| if x > cut_off { 1 } else { 0 })
        .collect::<Vec<_>>();
    Array::from_shape_vec((h, w), binary_arr).unwrap()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![processing, preview])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
