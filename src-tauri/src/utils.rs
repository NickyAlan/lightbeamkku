use tauri::Manager;
use std::collections::HashMap;
use std::u16;
use dicom::pixeldata::image::GrayImage;
use dicom::dictionary_std::tags::{self};
use ndarray::{s, Array, ArrayBase, Axis, Dim, OwnedRepr};
use dicom::object::{FileDicomObject, InMemDicomObject, Tag};
use dicom::{object::open_file, pixeldata::PixelDecoder};
use std::cmp::max;

type DcmObj = dicom::object::FileDicomObject<dicom::object::InMemDicomObject>;
pub type U16Array = ArrayBase<OwnedRepr<u16>, Dim<[usize; 2]>>;
pub type U8Array = ArrayBase<OwnedRepr<u8>, Dim<[usize; 2]>>;
pub type U128Array = ArrayBase<OwnedRepr<u128>, Dim<[usize; 2]>>;
type I32Array = ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>;
type Obj = FileDicomObject<InMemDicomObject>;

pub fn open_dcm_file(file_path: String) -> Option<DcmObj> {
    match open_file(file_path) {
        Ok(obj) => {
            println!("LOAD: OK");
            return Some(obj);
        }, 
        Err(_) => {
            println!("LOAD: ERR");
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

pub fn find_common_value(arr: U16Array, axis: u8) -> i32 {
    // find most common pixel value in specific axis
    // axis 0 = by col, axis 1 = by row
    // return: most common pixel value

    // [b:low , w:high] color (if inv use argmin)
    let values = argmax(arr, axis);
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

pub fn find_center_line(arr: U16Array) -> (i32, i32, i32, i32, f64) {
    // find horizontal center line
    // theta: alignment of the center line
    // return (x1, y1, x2, y2, theta)

    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let hp = (0.2 * h as f32) as i32;
    let wp = (0.06 * w as f32) as i32;
    
    let crop = [
        (hp),
        (h as i32 - hp),
        (wp),
        (wp*2)
    ];

    // left point
    let focus_l = arr.slice(s![
        crop[0]..crop[1], crop[2]..crop[3]
    ]).to_owned();
    // add hp (not start at 0)
    let y1 = find_common_value(focus_l, 0) + hp;
    // right point
    let focus_r = arr.slice(s![
        crop[0]..crop[1], w as i32 -crop[3]..w as i32 -crop[2]
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + hp;

    // theta
    let theta = find_theta(crop[2], w as i32-crop[3], y1, y2);

    // w - 1 for visualize not over blank
    (0, y1, w as i32 - 1, y2, theta)
}

/// find center mis-align theta from y1, y2
/// 
/// Return: theta in radius
pub fn find_theta(x1: i32, x2: i32, y1: i32, y2: i32) -> f64 {    
    let a = y2 - y1;
    let w = x2 - x1;
    let theta_r = (a as f64 / w as f64).atan();
    theta_r as f64 
}

/// rotate array CCW by theta in radius 
pub fn rotate_array(theta_r: f64, array: U16Array) -> U16Array {
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

            // Calculate the four surrounding pixel indices
            let x0 = new_x.floor() as isize;
            let x1 = x0 + 1;
            let y0 = new_y.floor() as isize;
            let y1 = y0 + 1;

            // Interpolate only if all indices are within bounds
            if x0 >= 0 && x1 < w as isize && y0 >= 0 && y1 < h as isize {
                let fx = new_x - x0 as f64;
                let fy = new_y - y0 as f64;

                let v00 = array[(y0 as usize, x0 as usize)] as f64;
                let v10 = array[(y0 as usize, x1 as usize)] as f64;
                let v01 = array[(y1 as usize, x0 as usize)] as f64;
                let v11 = array[(y1 as usize, x1 as usize)] as f64;

                // Bilinear interpolation formula
                let value = (1.0 - fx) * (1.0 - fy) * v00
                          + fx * (1.0 - fy) * v10
                          + (1.0 - fx) * fy * v01
                          + fx * fy * v11;

                rotated[(i, j)] = value.round() as u16;
            }
        }
    }

    rotated
}


/// find horizontal lines[y-axis]
/// 
/// Returns: (top(y_avg), center(y_avg), bottom(y_avg)) points
pub fn fint_horizontal_line(arr: U16Array) -> Vec<i32> {
    let mut ypoints = vec![];
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let hp = (0.27 * h as f32) as i32;
    let wp = (0.07 * w as f32) as i32;
    let offset = 20; // padding from rotating error (for top and bottom)

    // top line
    let crop = [offset, hp, wp*2, wp*3];
    let focus_l = arr.slice(s![
        crop[0]..crop[1], crop[2]..crop[3]
    ]).to_owned();
    let y1 = find_common_value(focus_l, 0) + crop[0];
    let focus_r= arr.slice(s![
        crop[0]..crop[1], w as i32 - crop[3]..w as i32 - crop[2]
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + crop[0];
    // average y1 and y2 (may be it not the same)
    ypoints.push((y1 + y2)/2);

    // center line
    let (_, y1, _, y2, _) = find_center_line(arr.clone());
    ypoints.push((y1 + y2)/2);
    
    // bottom line    
    let crop = [h as i32 - hp, h as i32 - offset, wp*4, wp*5];
    let focus_l = arr.slice(s![
        crop[0]..crop[1],crop[2]..crop[3]
    ]).to_owned();
    // start at  h as i32 - hp (not 0)
    let y1 = find_common_value(focus_l, 0) + crop[0];
    let focus_r = arr.slice(s![
        crop[0]..crop[1],w as i32 - wp*3..w as i32 - wp*2
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + crop[0];
    ypoints.push((y1 + y2)/2);

    ypoints
}

/// find vertical lines[x-axis]
/// 
/// Returns: (left(x_avg), center(x_avg), right(x_avg)) points
pub fn find_vertical_line(arr: U16Array) -> Vec<i32> {
    let mut xpoints = vec![];
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let hp = (0.05 * h as f32) as i32;
    let wp = (0.04 * w as f32) as i32;
    let offset = 10; // padding from rotating error (for left and right)

    // left line
    let crop = [hp*3, hp*7, wp+offset, wp*6];
    let focus_t = arr.slice(s![
        crop[0]..crop[1], crop[2]..crop[3]
    ]).to_owned();
    let x1 = find_common_value(focus_t, 1) + crop[2];
    let focus_b = arr.slice(s![
        h as i32 - crop[1]..h as i32 - crop[0], crop[2]..crop[3]
    ]).to_owned();
    let x2 = find_common_value(focus_b, 1) + crop[2];
    xpoints.push((x1+x2)/2);

    // right line
    let focus_t = arr.slice(s![
        crop[0]..crop[1], w as i32 - crop[3]..w as i32 - crop[2]
    ]).to_owned();
    let x1 = find_common_value(focus_t, 1) + w as i32 - crop[3];
    let focus_b = arr.slice(s![
        h as i32 - crop[1]..h as i32 - crop[0], w as i32 - crop[3]..w as i32 - crop[2]
    ]).to_owned();
    let x2 = find_common_value(focus_b, 1) + w as i32 - crop[3];
    xpoints.push((x1+x2)/2);

    // center line
    let hp = (0.18 * h as f32) as i32;
    let wp = (0.20 * w as f32) as i32;
    
    let crop = [hp, hp*2, wp*2, wp*3];
    let focus_t = arr.slice(s![
        crop[0]..crop[1], crop[2]..crop[3]
    ]).to_owned();
    let x1 = find_common_value(focus_t, 1) + crop[2];
    let focus_b = arr.slice(s![
        h as i32 - crop[1].. h as i32 - crop[0], crop[2]..crop[3]
    ]).to_owned();
    let x2 = find_common_value(focus_b, 1) + crop[2];
    xpoints.push((x1+x2)/2);
    
    // swap bottom and center in xpoints
    let centerpoint = xpoints[2];
    xpoints[2] = xpoints[1];
    xpoints[1] = centerpoint;
    xpoints
}

pub fn argmin(arr: U16Array, axis: u8) -> Vec<usize> {
    // argmin of each column(axis=0), row(axis=1)
    // return position that has minimum pixel value 
    let rows = arr.nrows();
    let cols = arr.ncols();
    let mut argmins = vec![];
    // argmin
    if axis == 0 {
        for c in 0..cols {
            let mut min_val_col = u16::MAX;
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
            let mut min_val_row = u16::MAX;
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

pub fn argmax(arr: U16Array, axis: u8) -> Vec<usize> {
    // argmax of each column(axis=0), row(axis=1)
    // return position that has minimum pixel value 
    let rows = arr.nrows();
    let cols = arr.ncols();
    let mut argmax = vec![];

    if axis == 0 {
        for c in 0..cols {
            let mut max_val_col = 0;
            let mut argmax_col = 0;
            for r in 0..rows {
                let pixel_value = arr[(r, c)];
                if pixel_value > max_val_col {
                    max_val_col = pixel_value;
                    argmax_col = r;
                }
            }
            argmax.push(argmax_col);
        }
    } else {
        for r in 0..rows {
            let mut max_val_row = 0;
            let mut argmax_row = 0;
            for c in 0..cols {
                let pixel_value = arr[(r, c)];
                if pixel_value > max_val_row {
                    max_val_row = pixel_value;
                    argmax_row = c;
                }
            }
            argmax.push(argmax_row);
        }
    }

    argmax 
}

/// convert number of pixel to centimeter as aspect ratio
pub fn pixel2cm(ypoints: &Vec<i32>, number_pixels: i32, is_rotate: bool) -> f32 {
    let _cm = (ypoints[2] - ypoints[1]) as f32;
    let mut ratio = 7.0;
    if is_rotate {
        ratio = 9.0;
    }
    // 100.0 for 2 decimal round
    (number_pixels as f32 *ratio*100.0/_cm).round() / 100.0
}

//// convert centimeter to number of pixel as aspect ratio
pub fn cm2pixel(ypoints: &Vec<i32>, cm: f32, is_rotate: bool) -> i32 {   
    let _cm = (ypoints[2] - ypoints[1]) as f32;
    let mut ratio = 7.0;
    if is_rotate {
        ratio = 9.0;
    }
    (_cm*cm/ratio).round() as i32
}

//// find box of each xs, ys for croping edges area
pub fn boxs_posision(xpoints: &Vec<i32>, ypoints: &Vec<i32>, arr: U16Array) -> Vec<[[i32; 2]; 2]> {
    // return box position(top-left(x, y), bottom-right(x, y))
    // Left, Right, Top, Bottom
    let shape = arr.shape();
    let h = shape[0];
    let w = shape[1];
    let rotate_err = (w as f32 * 0.01) as i32;
    let mut pos = vec![];

    // Left-Right
    let add_hor_crop = (w as f32 * 0.1) as i32;
    let add_ver_crop = (h as f32 * 0.06) as i32;
    // Left
    let left_p = [xpoints[0], (ypoints[0] + ypoints[1])/2];
    let top_left_point = [max(left_p[0]-add_hor_crop, rotate_err), left_p[1]-add_ver_crop];
    let bottom_right_point = [left_p[0]+add_hor_crop, left_p[1]+add_ver_crop];
    pos.push([top_left_point, bottom_right_point]);

    // Right
    let right_p = [xpoints[2], (ypoints[0] + ypoints[1]) / 2];
    let top_left_point = [right_p[0] - add_hor_crop, right_p[1] - add_ver_crop];
    let bottom_right_point = [
        (right_p[0] + add_hor_crop).min(w as i32 - rotate_err - 1),
        right_p[1] + add_ver_crop,
    ];
    pos.push([top_left_point, bottom_right_point]);

    // Top-Bottom
    // Top
    let top_p = [(xpoints[1] + xpoints[2]) / 2, ypoints[0]];
    let top_left_point = [
        top_p[0] - add_ver_crop,
        (top_p[1] - add_hor_crop).max(rotate_err),
    ];
    let bottom_right_point = [top_p[0] + add_ver_crop, top_p[1] + add_hor_crop];
    pos.push([top_left_point, bottom_right_point]);

    // Bottom
    let bottop_p = [(xpoints[1] + xpoints[2]) / 2, ypoints[2]];
    let top_left_point = [
        bottop_p[0] - add_ver_crop,
        bottop_p[1] - add_hor_crop,
    ];
    let bottom_right_point = [
        bottop_p[0] + add_ver_crop,
        (bottop_p[1] + add_hor_crop).min(h as i32 - rotate_err - 1),
    ];
    pos.push([top_left_point, bottom_right_point]);

    pos
}

pub fn get_crop_area(positions: Vec<[[i32; 2]; 2]>, arr: U16Array) -> [U16Array; 4]{
    // for left, right, top, bottom
    // get crop area pixels from the top_left_point, bottom_right_point
    
    let focuses: Vec<_> = positions.iter()
        .map(|[top_left_point, bottom_right_point]| {
            arr.slice(s![
                top_left_point[1]..bottom_right_point[1],
                top_left_point[0]..bottom_right_point[0]
            ])
            .to_owned()
        })
        .collect();

    focuses.try_into().unwrap()
}

pub fn find_edges_pos(crop_areas: [U16Array; 4], boxs_pos: Vec<[[i32; 2]; 2]>) -> Vec<i32> {
    let mut edges_pos = vec![];
    let mut by_x;
    for (q, crop_area) in crop_areas.into_iter().enumerate() {
        by_x = q <= 1; // x-axis(0, 1), y-axis(2, 3)
        // central diff
        let mut edge_pos = central_diff(crop_area, by_x) as i32;
        // adjust to the image
        let [top_left, bottom_right] = boxs_pos[q];
        // x-axis: add x
        if q<=1 {
            edge_pos = edge_pos + top_left[0];
        } else {
            edge_pos = edge_pos + top_left[1];
        }
        edges_pos.push(edge_pos);
    }
    edges_pos
}

fn central_diff(pixels: U16Array, by_x: bool) -> usize {
    // find most difference position
    // by_x(True, False) = (x, y)
    let nrows = pixels.nrows();
    let ncols = pixels.ncols();
    let mut edge_pixels = vec![];

    if by_x {
        // fininte difference by cols
        for r_idx in 0..nrows {
            let mut edge_pixels_col = vec![];
            for c_idx in 1..ncols-1 {
                let first_val = pixels[(r_idx, c_idx-1)];
                let second_val = pixels[(r_idx, c_idx+1)];
                let diff_val = (first_val as i32 - second_val as i32).abs();
                edge_pixels_col.push(diff_val as u16);
            }
            edge_pixels.push(edge_pixels_col);
        }
    } else {
        for c_idx in 0..ncols {
            let mut edge_pixels_row = vec![];
            for r_idx in 1..nrows-1 {
                let first_val = pixels[(r_idx-1, c_idx)];
                let second_val = pixels[(r_idx+1, c_idx)];
                let diff_val = (first_val as i32 - second_val as i32).abs();
                edge_pixels_row.push(diff_val as u16);
            }
            edge_pixels.push(edge_pixels_row);
        }
    }

    let mut med_edge = median_by_col(edge_pixels);
    let [first_pos, last_pos] = bounder_percentile(&mut med_edge, 99.0);
    let avg_pos = (first_pos + last_pos)/2;
    let edge_pos = avg_pos + 1;
    edge_pos
}

fn median_of_column(column: &mut Vec<u16>) -> f32 {
    column.sort();  // Sort the column values
    let len = column.len();

    if len % 2 == 1 {
        column[len / 2] as f32  // Odd number of elements, take the middle one
    } else {
        (column[len / 2 - 1] as f32 + column[len / 2] as f32) / 2.0  // Even number of elements, average the middle two
    }
}

fn median_by_col(arr: Vec<Vec<u16>>) -> Vec<f32> {
    let ncols = arr[0].len();
    let mut medians = vec![];
    for col in 0..ncols {
        let mut column_values: Vec<u16> = arr.iter().map(|row| row[col]).collect();
        let median = median_of_column(&mut column_values);
        medians.push(median);
    }
    medians
}

fn bounder_percentile(arr: &Vec<f32>, q: f32) -> [usize; 2] {
    // find boundery of values of average edge 
    let ts = percentile(arr, q);
    let mut first_pos = 0;
    let mut last_pos = 0;
    for (pos, val) in arr.into_iter().enumerate() {
        if *val as f32 >= ts {
            first_pos = pos;
            break;
        }
    }

    let n = arr.len();
    for pos in 0..n {
        let val = arr[n-pos-1];
        if val >= ts {
            last_pos = n-pos-1;
            break;
        }
    }
    
    [first_pos, last_pos]
}

fn percentile(arr: &Vec<f32>, q: f32) -> f32 {
    let mut sorted_arr = arr.clone();  // Make a copy of the vector to avoid modifying the original
    sorted_arr.sort_by(|a, b| a.partial_cmp(b).unwrap());  // Sort the copied array in ascending order

    let n = sorted_arr.len();
    let rank = q / 100.0 * (n as f32 - 1.0);  // Calculate the rank (index) for the percentile

    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        sorted_arr[lower_index]  // If the rank is an integer, return the value at that index
    } else {
        // If the rank is between two indices, interpolate between them
        let lower_value = sorted_arr[lower_index];
        let upper_value = sorted_arr[upper_index];
        lower_value + (upper_value - lower_value) * (rank - rank.floor())  // Interpolate
    }
}

pub fn get_result(edges_pos: Vec<i32>, xpoints: &Vec<i32>, ypoints: &Vec<i32>) -> ([i32; 2], [[i32; 4]; 4], [f32; 4], [f32; 4]){
    // get result from edge_positons
    let center_p = [xpoints[1], ypoints[1]];
    // x1, y1, x2, y2
    let res_xy = [
        [edges_pos[0], edges_pos[2], edges_pos[0], edges_pos[3]],
        [edges_pos[1], edges_pos[2], edges_pos[1], edges_pos[3]],
        [edges_pos[0], edges_pos[2], edges_pos[1], edges_pos[2]],
        [edges_pos[0], edges_pos[3], edges_pos[1], edges_pos[3]]
    ];
    let res_length = [
        pixel2cm(&ypoints, center_p[0]-edges_pos[0], false),
        pixel2cm(&ypoints, edges_pos[1]-center_p[0], false),
        pixel2cm(&ypoints, center_p[1]-edges_pos[2], false),
        pixel2cm(&ypoints, edges_pos[3]-center_p[1], false)
    ];
    let res_err = [
        ((9.0 - res_length[0]) * 100.0).round() / 100.0,
        ((9.0 - res_length[1]) * 100.0).round() / 100.0,
        ((7.0 - res_length[2]) * 100.0).round() / 100.0,
        ((7.0 - res_length[3]) * 100.0).round() / 100.0,
    ];

    (center_p, res_xy, res_length, res_err)
}