// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod utils;
use crate::utils::{open_dcm_file, save_to_image, get_detail, convert_to_u8, save_to_image_u8, find_common_value, find_center_line, find_theta, rotate_array, fint_horizontal_line, find_vertical_line, boxs_posision, find_edges_pos, split_q_circle, farthest_q, center_point};
use crate::utils::{U8Array, U16Array, pixel2cm, cm2pixel, U128Array, find_edge_tool, find_mean, cast_type_arr, inv_lut, rectangle_edge_points, length_line, distance_pixel};
use std::collections::HashMap;
use ndarray_stats::QuantileExt;
use dicom::pixeldata::image::{flat, GrayImage};
use dicom::dictionary_std::tags::{self, FOCAL_DISTANCE, NUMBER_OF_COMPENSATORS, SPHERE_POWER};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::object::{pixeldata, FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};
use utils::{argmax, get_crop_area};

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
            let [row1, row2, col1, col2] = arr_correction(arr.clone());
            let mut arr = arr.slice(s![
                row1..row2, col1..col2
            ]).to_owned();
            let mut h = arr.nrows();
            let mut w = arr.ncols();
            // check is rotate
            let mut is_rotate = false;
            if (row2-row1) > (col2-col1) {
                is_rotate = true;
                arr = rotate_array(3.14/2.0, arr.clone());
                h = arr.nrows();
                w = arr.ncols();
            }
            // check is inv
            let mut is_inv =  false;
            let hp = (0.2*(h as f32)) as usize;
            let wp = (0.06*(w as f32)) as usize;
            let focus_l = arr.slice(s![hp..h-hp, wp*2..wp*3]).to_owned();
            let values = argmax(focus_l.clone(), 0);
            let mut counts: HashMap<usize, u16> = HashMap::new();
            for n in &values {
                let count = counts.entry(*n).or_insert(0);
                *count += 1;
            }
            let mut max_key = None;
            let mut max_val = std::u16::MIN;
            for (k, v) in counts {
                if v > max_val {
                    max_key = Some(k);
                    max_val = v;
                }
            }
            if (max_val as f32/focus_l.ncols() as f32) < 0.3 {
                is_inv = true; 
                arr = inv_lut(arr.clone());
            }


            // Find Center Line
            let (_, _, _, _, theta_r) = find_center_line(arr.clone());
            // // Adjust angle
            let rotated_arr = rotate_array(theta_r, arr.clone());
            // Fine Lines in Rotated array
            let ypoints = fint_horizontal_line(rotated_arr.clone());
            let xpoints = find_vertical_line(rotated_arr);
            
            // // Small field
            match open_dcm_file(small_field) {
                Some(obj) => {
                    let pixel_data: dicom::pixeldata::DecodedPixelData<'_> = obj.decode_pixel_data().unwrap();
                    let arr = pixel_data.to_ndarray::<u16>().unwrap().slice(s![0, .., .., 0]).to_owned();
                    let mut arr = arr.slice(s![
                        row1..row2, col1..col2
                    ]).to_owned();
                    // is_rotate and is_inv
                    if is_rotate {
                        arr = rotate_array(3.14/2.0, arr.clone());
                    }
                    if is_inv {
                        arr = inv_lut(arr.clone());
                    }

                    let rotated_arr2 = rotate_array(theta_r, arr);
                    
                    // save_to_image(rotated_arr2.clone(), "c:/Users/alant/Desktop/arr2.jpg".to_string());
                    
                    // Find the Edges
                    // boxs_position(area for crop)
                    let boxs_pos = boxs_posision(&xpoints, &ypoints, rotated_arr2.clone());
                    // get crop area
                    let crop_areas = get_crop_area(boxs_pos.clone(), rotated_arr2.clone());
                    // // edges positions
                    let xypoints = [xpoints[0], xpoints[0], xpoints[2], xpoints[2], ypoints[0], ypoints[0], ypoints[2], ypoints[2]];
                    let edges_pos = find_edges_pos(crop_areas, boxs_pos.clone(), xypoints, &ypoints);
                    // find 4 points(top-left[x, y], top-right, bottom_left, bottom-right) of the edges
                    let (points, mbs) = rectangle_edge_points(boxs_pos, edges_pos);
                    // // Result: left, right, top, bottom [x1, y1, x2, y2, length]
                    let (results, results_pos) = length_line(points, mbs, &xpoints, &ypoints);

                    // Fine the circles
                    let (q_arr, white_ts, (xc, yc)) = split_q_circle(&xpoints, &ypoints, rotated_arr2);
                    let (farthest_q, farthest_point) = farthest_q(q_arr.clone(), white_ts);
                    let (x, y) = center_point(farthest_point, farthest_q, xc, yc);
                    dbg!(x, y, xc, yc);
                    let cir_distance = distance_pixel(x, y, xc as usize, yc as usize);
                    dbg!(results, results_pos);
                    // dbg!(center_p, res_xy, res_length, res_err);
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

fn arr_correction(mut arr: U16Array) -> [usize; 4] {
    // crop array as expect.
    // Find Test-Tool
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let offset = 30;
    // find x-axis
    let focus_x1 = arr.slice(s![
        (h/2)-offset..h/2, offset..w/2
    ]).to_owned();
    let focus_x1_u128 = cast_type_arr(focus_x1);
    let focus_x1_avg = focus_x1_u128.mean_axis(Axis(0)).unwrap().into_raw_vec();
    let n = focus_x1_avg.len();
    let ts = find_mean(focus_x1_avg.clone(), n) as u128;
    let x1 = find_edge_tool(focus_x1_avg, n, offset, ts) + offset;

    let focus_y1 = arr.slice(s![
        offset..h/2, w/3..(w/3)+offset
    ]).to_owned();
    let focus_y1_u128 = cast_type_arr(focus_y1);
    let focus_y1_avg = focus_y1_u128.mean_axis(Axis(1)).unwrap().into_raw_vec();
    let n = focus_y1_avg.len();
    let ts = find_mean(focus_y1_avg.clone(), n) as u128;
    let y1 = find_edge_tool(focus_y1_avg, n, offset, ts) + offset;

    let focus_x2 = arr.slice(s![
        h/2-offset..h/2, w/2..w-offset
    ]).to_owned();
    let focus_x2_u128 = cast_type_arr(focus_x2);
    let mut focus_x2_avg = focus_x2_u128.mean_axis(Axis(0)).unwrap().into_raw_vec();
    focus_x2_avg.reverse();
    let n = focus_x2_avg.len();
    let ts = find_mean(focus_x2_avg.clone(), n) as u128;
    let x2 = n - find_edge_tool(focus_x2_avg, n, offset, ts) + w/2;

    let focus_y2 = arr.slice(s![
        h/2..h-offset, w/3..(w/3)+offset
    ]).to_owned();
    let focus_y2_u128 = cast_type_arr(focus_y2);
    let mut focus_y2_avg = focus_y2_u128.mean_axis(Axis(1)).unwrap().into_raw_vec();
    focus_y2_avg.reverse();
    let n = focus_y2_avg.len();
    let ts = find_mean(focus_y2_avg.clone(), n) as u128;
    let y2 = n - find_edge_tool(focus_y2_avg, n, offset, ts) + h/2;
    
    [y1, y2, x1, x2]
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
