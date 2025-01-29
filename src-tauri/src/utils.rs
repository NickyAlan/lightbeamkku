use ndarray_stats::QuantileExt;
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
    let max_v = array.max().unwrap().clone() as u16;
    // let mut rotated = ndarray::Array::zeros((h as usize, w as usize));
    let mut rotated = Array::from_elem((h as usize, w as usize), max_v);
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
    } else if axis == 1 {
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

fn argmax_1d(arr: U16Array) -> i32 {
    let arr = arr.into_raw_vec();
    let n = arr.len();
    let mut max_v = 0;
    let mut argmax = 0;
    for i in 0..n {
        if arr[i] > max_v {
            max_v = arr[i];
            argmax = i
        }
    }

    argmax as i32
}

fn argmax_vec(vector: Vec<f32>) -> (usize, f32) {
    let n = vector.len();
    let mut max_v = 0.0;
    let mut argmax = 0;
    for i in 0..n {
        if vector[i] > max_v {
            max_v = vector[i];
            argmax = i;
        }
    }
    (argmax, max_v as f32)
}

/// convert number of pixel to centimeter as aspect ratio
pub fn pixel2cm(ypoints: &Vec<i32>, number_pixels: i32) -> f32 {
    let _cm = (ypoints[2] - ypoints[1]) as f32;
    let ratio = 7.0;
    // 1000.0 for 3 decimal round
    (number_pixels as f32 *ratio*1000.0/_cm).round() / 1000.0
}

//// convert centimeter to number of pixel as aspect ratio
pub fn cm2pixel(ypoints: &Vec<i32>, cm: f32) -> i32 {   
    let _cm = (ypoints[2] - ypoints[1]) as f32;
    let ratio = 7.0;
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
    let add_hor_crop = (w as f32 * 0.15) as i32;
    let add_ver_crop = (h as f32 * 0.08) as i32;
    // Left1
    let left_p = [xpoints[0], (ypoints[0] as f32 *3./4. + ypoints[1] as f32 /4.) as i32];
    let top_left_point = [max(left_p[0]-add_hor_crop, rotate_err), left_p[1]-add_ver_crop];
    let bottom_right_point = [left_p[0]+add_hor_crop, left_p[1]+add_ver_crop];
    pos.push([top_left_point, bottom_right_point]);
    // Left2
    let left_p = [xpoints[0], (ypoints[1] as f32 /4. + ypoints[2] as f32 * 3./4.) as i32];
    let top_left_point = [max(left_p[0]-add_hor_crop, rotate_err), left_p[1]-add_ver_crop];
    let bottom_right_point = [left_p[0]+add_hor_crop, left_p[1]+add_ver_crop];
    pos.push([top_left_point, bottom_right_point]);

    // Right1
    let right_p = [xpoints[2], (ypoints[0] as f32 * 3./4. + ypoints[1] as f32 / 4.) as i32];
    let top_left_point = [right_p[0] - add_hor_crop, right_p[1] - add_ver_crop];
    let bottom_right_point = [
        (right_p[0] + add_hor_crop).min(w as i32 - rotate_err - 1),
        right_p[1] + add_ver_crop,
    ];
    pos.push([top_left_point, bottom_right_point]);

    // Right2
    let right_p = [xpoints[2], (ypoints[1] as f32 / 4. + ypoints[2] as f32 * 3./4.) as i32];
    let top_left_point = [right_p[0] - add_hor_crop, right_p[1] - add_ver_crop];
    let bottom_right_point = [
        (right_p[0] + add_hor_crop).min(w as i32 - rotate_err - 1),
        right_p[1] + add_ver_crop,
    ];
    pos.push([top_left_point, bottom_right_point]);

    // Top-Bottom
    // Top1
    let top_p = [(xpoints[1] as f32 * 6./8. + xpoints[0] as f32 * 1./4.) as i32, ypoints[0]];
    let top_left_point = [
        top_p[0] - add_ver_crop,
        (top_p[1] - add_hor_crop).max(rotate_err),
    ];
    let bottom_right_point = [top_p[0] + add_ver_crop, top_p[1] + add_hor_crop];
    pos.push([top_left_point, bottom_right_point]);

    // Top2
    let top_p = [(xpoints[1] as f32 / 4. + xpoints[2] as f32 * 3./4.) as i32, ypoints[0]];
    let top_left_point = [
        top_p[0] - add_ver_crop,
        (top_p[1] - add_hor_crop).max(rotate_err),
    ];
    let bottom_right_point = [top_p[0] + add_ver_crop, top_p[1] + add_hor_crop];
    pos.push([top_left_point, bottom_right_point]);

    // Bottom1
    let bottop_p = [(xpoints[1] as f32 * 3./8. + xpoints[0] as f32 * 5./8.) as i32, ypoints[2]];
    let top_left_point = [
        bottop_p[0] - add_ver_crop,
        bottop_p[1] - add_hor_crop,
    ];
    let bottom_right_point = [
        bottop_p[0] + add_ver_crop,
        (bottop_p[1] + add_hor_crop).min(h as i32 - rotate_err - 1),
    ];
    pos.push([top_left_point, bottom_right_point]);

    // Bottom2
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

pub fn get_crop_area(positions: Vec<[[i32; 2]; 2]>, arr: U16Array) -> [U16Array; 8]{
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

pub fn find_edges_pos(crop_areas: [U16Array; 8], boxs_pos: Vec<[[i32; 2]; 2]>, xypoints: [i32; 8], ypoints: &Vec<i32>) -> Vec<i32> {
    let mut edges_pos = vec![];
    let mut by_x;
    for (q, crop_area) in crop_areas.into_iter().enumerate() {
        by_x = q <= 3; 
        // adjust to the image
        let [top_lefts, _] = boxs_pos[q];
        let mut top_left;
        if by_x {
            top_left = top_lefts[0];
        } else {
            top_left = top_lefts[1];
        }
        // central diff
        let mut edge_pos = central_diff(crop_area, top_left, xypoints[q], by_x, ypoints) as i32;
        // x-axis: add x
        edge_pos = edge_pos + top_left;
        edges_pos.push(edge_pos);
    }
    edges_pos
}

fn central_diff(pixels: U16Array, top_left: i32, xypoint: i32, by_x: bool, ypoints: &Vec<i32>) -> usize {
    // find most difference position
    // by_x(True, False) = (x, y)
    let nrows = pixels.nrows();
    let ncols = pixels.ncols();
    let mut edge_pixels = vec![];
    let adjust_pos = (xypoint - top_left) as usize;
    let half_line_w = cm2pixel(ypoints, 0.04) as usize;

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
    // remove actual line
    let new_val = med_edge[adjust_pos - half_line_w - 1].clone();
    let start = adjust_pos - half_line_w;
    let end = adjust_pos + half_line_w;
    for i in start..end {
        med_edge[i] = new_val;
    }
    
    let (peak_loc, half_peak) = find_peak(med_edge.clone());
    
    // find edge not the actual line
    let far_pixel = cm2pixel(ypoints, 0.28) as usize;
    let left_start = peak_loc - far_pixel;
    let left_walk = left_start + 1;
    let right_start = peak_loc + far_pixel;
    let right_walk = med_edge.len() - right_start;
    let range_line = [adjust_pos-half_line_w*2, adjust_pos+half_line_w*2];
    let mut edge_pos = peak_loc + 1;

    // right check
    for i in 0..right_walk {
        let cur_loc = right_start + i;
        let pixel_val = med_edge[cur_loc];
        if pixel_val >= half_peak {
            if !(range_line[0] <= cur_loc && cur_loc < range_line[1]) {
                let find_range = [cur_loc, cur_loc+(half_line_w*2)];
                edge_pos = find_range[0] + argmax_vec(med_edge[find_range[0]..find_range[1]].to_vec()).0 + 1;
            }
            break;
        }
    }

    // left check
    for i in 0..left_walk {
        let cur_loc = left_walk - i;
        let pixel_val = med_edge[cur_loc];
        if pixel_val >= half_peak {
            if !(range_line[0] <= cur_loc && cur_loc < range_line[1]) {
                let find_range = [cur_loc-(half_line_w*2), cur_loc];
                edge_pos = find_range[0] + argmax_vec(med_edge[find_range[0]..find_range[1]].to_vec()).0 + 1;
            }
            break;
        }
    }

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

fn find_peak(arr: Vec<f32>) -> (usize, f32) {
    let (peak_loc, max_v) = argmax_vec(arr);
    let half_peak = max_v * 0.7;
    (peak_loc, half_peak)
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


pub fn split_q_circle(xpoints: &Vec<i32>, ypoints: &Vec<i32>, arr: U16Array) -> (U16Array, [U16Array; 4], f32, (i32, i32)) {
    // split the circle into 4q 
    let one_cm_pixel = cm2pixel(ypoints, 0.9);
    let xx = [xpoints[1]-one_cm_pixel, xpoints[1]+one_cm_pixel];
    let yy = [ypoints[1]-one_cm_pixel, ypoints[1]+one_cm_pixel];
    let circle_arr = arr.slice(s![
        yy[0]..yy[1], xx[0]..xx[1]
    ]).to_owned();
    let inner_r = cm2pixel(ypoints, 0.41);
    let outter_r = cm2pixel(ypoints, 0.71);
    
    // DEBUG
    // let circle_arr = rotate_array(3.14, circle_arr);

    // split 4q
    let mut cir_f32 = vec![];
    for v in circle_arr.clone().into_raw_vec() {cir_f32.push(v as f32);}
    let white_ts = percentile(&cir_f32, 99.0);

    let [xc, yc] = find_center_circle_line(circle_arr.clone());
    let top_left = circle_arr.slice(s![
        ..yc, ..xc
    ]).to_owned();
    let top_right = circle_arr.slice(s![
        ..yc, xc+1.. 
    ]).to_owned();
    let bottom_left = circle_arr.slice(s![
        yc+1.., ..xc
    ]).to_owned();
    let bottom_right = circle_arr.slice(s![
        yc+1.., xc+1..
    ]).to_owned();
    
    (circle_arr, [top_left, top_right, bottom_left, bottom_right], white_ts, (xc, yc))
}

fn find_center_circle_line(arr: U16Array) -> [i32; 2] {
    let shape = arr.shape();
    let h = shape[0] as i32;
    let w = shape[1] as i32;
    let hp = (0.2 * h as f32) as i32;
    let wp = (0.05 * w as f32) as i32;

    // left
    let focus_l = arr.slice(s![
        hp..h-hp, wp..wp*4
    ]).to_owned();
    let y1 = find_common_value(focus_l, 0) + hp;
    // right
    let focus_r = arr.slice(s![
        hp..h-hp, w-(wp*2)..w
    ]).to_owned();
    let y2 = find_common_value(focus_r, 0) + hp;
    let y = ((y1 as f32 + y2 as f32)/2.0).ceil() as i32;

    // top
    let focus_t = arr.slice(s![
        wp..wp*3, hp..h-hp
    ]).to_owned();
    let x1 = find_common_value(focus_t, 1) + hp;
    // bottom
    let focus_b = arr.slice(s![
        w-(wp*2)..w, hp..h-hp
    ]).to_owned();
    let x2 = find_common_value(focus_b, 1) + hp;
    let x = ((x1 as f32 + x2 as f32)/2.0).ceil() as i32;

    [x, y]
}

pub fn farthest_q(q_arr: [U16Array; 4], white_ts: f32) -> (usize, [[usize; 2]; 2]) {
    // q_array = [top_left, top_right, bottom_left, bottom_right]
    // fartest point quadrate 
    // return fartest quadrate, [row, col]
    let mut farthest_q = 0;
    let mut farthest = 0.0;
    let (mut row_idx, mut col_idx) = (0, 0);
    let white_ts = white_ts as u16;
    // is first row, col
    let config = [
        (false, false),
        (false, true),
        (true, false),
        (true, true)
    ];
    for (i, q) in q_arr.clone().into_iter().enumerate() {
        let (x, y) = find_farthest_white(q.clone(), config[i].0, config[i].1, white_ts);
        let d = ((x.pow(2) + y.pow(2)) as f64).sqrt();
        if d >= farthest {
            farthest = d;
            farthest_q = i;
            (row_idx, col_idx) = (x, y);
        }
    }

    let mut farthest_point = [[row_idx as usize, 0], [0, col_idx as usize]];
    let shape = q_arr[farthest_q].shape(); 
    let nrows = shape[0] as i32;
    let ncols = shape[1] as i32;
    // // find corner of the point: find p_max col for row, ...
    match farthest_q {
        0 => {
            // max_col for row
            let slice_row = q_arr[farthest_q].slice(s![nrows - row_idx - 1, (ncols - col_idx - 1)..]).to_owned();
            let reversed_slice_row: Vec<u16> = slice_row.iter().rev().cloned().collect();
            farthest_point[0][1] = reversed_slice_row.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);

            // max_row for col
            let slice_col = q_arr[farthest_q].slice(s![(nrows - row_idx - 1).., ncols - col_idx - 1]).to_owned();
            let reversed_slice_col: Vec<u16> = slice_col.iter().rev().cloned().collect();
            farthest_point[1][0] = reversed_slice_col.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);
        }
        1 => {
            // max_col for row
            let slice_row = q_arr[farthest_q].slice(s![nrows - row_idx - 1, ..=col_idx]).to_owned();
            farthest_point[0][1] = slice_row.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);

            // max_row for col
            let slice_col = q_arr[farthest_q].slice(s![(nrows - row_idx - 1).., col_idx]).to_owned();
            let reversed_slice_col: Vec<u16> = slice_col.iter().rev().cloned().collect();
            farthest_point[1][0] = reversed_slice_col.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);
        }
        2 => {
            // max_col for row
            let slice_row = q_arr[farthest_q].slice(s![row_idx, (ncols - col_idx - 1)..]).to_owned();
            let reversed_slice_row: Vec<u16> = slice_row.iter().rev().cloned().collect();
            farthest_point[0][1] = reversed_slice_row.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);

            // max_row for col
            let slice_col = q_arr[farthest_q].slice(s![..=row_idx, ncols - col_idx - 1]).to_owned();
            farthest_point[1][0] = slice_col.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);
        }
        3 => {
            // max_col for row
            let slice_row = q_arr[farthest_q].slice(s![row_idx, ..=col_idx]).to_owned();
            farthest_point[0][1] = slice_row.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);

            // max_row for col
            let slice_col = q_arr[farthest_q].slice(s![..=row_idx, col_idx]).to_owned();
            farthest_point[1][0] = slice_col.iter().enumerate().max_by(|a, b| a.1.cmp(b.1)).map(|(i, _)| i).unwrap_or(0);
        }
        _ => {}
    }

    (farthest_q, farthest_point)
}

fn find_farthest_white(arr: U16Array, first_row: bool, first_col: bool, white_ts: u16) -> (i32, i32) {
    // find find_farthest_white return (row, col)
    let nrows = arr.nrows();
    let ncols = arr.ncols();
    let mut farthest_col = 0;
    // find cols first
    for row in 0..nrows {
        for col in 0..ncols {
            let new_row = if first_row { row } else { nrows - row - 1};
            let new_col = if first_col { col } else { ncols - col - 1};
            let p_val = arr[(new_row, new_col)];
            if p_val >= white_ts {
                if col > farthest_col {
                    farthest_col = col;
                }
            } 
        }
    }

    let mut farthest_row = 0;
    for col in 0..ncols {
        for row in 0..nrows {
            let new_row = if first_row { row } else { nrows - row - 1 };
            let new_col = if first_col { col } else { ncols - col - 1 };
            let p_val = arr[(new_row, new_col)];
            if p_val >= white_ts {
                if row > farthest_row {
                    farthest_row = row;
                }
            } 
        }
    }

    (farthest_row as i32, farthest_col as i32) 
}

pub fn center_point(farthest_point: [[usize; 2]; 2], q: usize, xc: i32, yc: i32) -> (usize, usize) {
    // defined center point from fartest point
    // return x, y
    let mut x = farthest_point[0][1] as i32;
    let mut y = farthest_point[1][0] as i32;
    if q == 0 {
        x = xc-1 - x;
        y = yc-1 - y;
    } else if q == 1 {
        x = xc+1 + x;
        y = yc-1 - y;
    } else if q == 2 {
        x = xc-1 - x;
        y = yc+1 + y;
    } else if q == 3 {
        x = xc+1 + x;
        y = yc+1 + y;
    }

    (x as usize, y as usize)
}

pub fn find_edge_tool(vector: Vec<u128>, n: usize, offset: usize, ts: u128) -> usize{
    let mut start_vals = vec![];
    let mut v = 0;
    for i in 0..offset {
        if ts >= vector[i] {
            v = 1;
        } else {
            v = 0;
        }
        start_vals.push(v);
    }

    let start_val = find_mean(start_vals.clone(), start_vals.len());
    let edge_ts = 10;
    let mut cur_edge = 0;
    let mut edge_pos = 0;
    let mut p_val = 0;
    for i in offset..n {
        if ts >= vector[i] {
            p_val = 1;
        } else {
            p_val = 0;
        }
        if p_val != start_val {
            cur_edge += 1;
        } else {
            cur_edge = 0;
        }
        if cur_edge >= edge_ts {
            edge_pos = i - edge_ts;
            break;
        }
    }

    edge_pos
}

pub fn find_mean(vector: Vec<u128>, n: usize) -> u16{
    let sum: u128 = vector.iter().map(|&x| x as u128).sum();
    let mean = sum as f64 / n as f64;
    mean as u16
}

pub fn cast_type_arr(arr: U16Array) -> U128Array{
    let h = arr.nrows();
    let w = arr.ncols();
    let arr_u128_vec = arr.iter()
        .map(|&x| x as u128)
        .collect::<Vec<_>>();
    let arr_u128 = Array::from_shape_vec((h, w), arr_u128_vec).unwrap();
    arr_u128
}

pub fn inv_lut(arr: U16Array) -> U16Array{
    let max_pixel = arr.max().unwrap();
    let min_pixel = arr.min().unwrap();
    let ncols = arr.ncols();
    let nrows = arr.nrows();
    let mut inv_arr = arr.clone();
    for i in 0..nrows{
        for j in 0..ncols {
            let val = arr[(i, j)];
            let new_val = max_pixel - val + min_pixel;
            inv_arr[(i, j)] = new_val;
        }
    }

    inv_arr
}

fn linear_equation(x1: i32, y1: i32, x2: i32, y2: i32) -> [f32; 2] {
    // prevent divided by zero
    let m;
    if x2 == x1 {
        m = (y2 - y1) as f32 ;
    } else {
        m = (y2 - y1) as f32 / (x2- x1) as f32
    }
    let b = y1 as f32 - m * x1 as f32;
    [m, b]
}

fn find_intersection(m1: f32, b1: f32, m2:f32, b2: f32) -> [i32; 2] {
    // find intersection point(x, y) of 2 lines (m, b)
    let x = (b2 - b1) / (m1 - m2);
    let y = m1 * x + b1;
    [x.round() as i32, y.round() as i32]
}

pub fn rectangle_edge_points(boxs_pos: Vec<[[i32; 2]; 2]>, edges_pos: Vec<i32>) -> ([[i32; 2];4], [[f32; 2];4]) {
    // find 4 points(top-left[x, y], top-right, bottom_left, bottom-right) of the edges
    let ly = [(boxs_pos[0][0][1] + boxs_pos[0][1][0])/2, (boxs_pos[1][0][1] + boxs_pos[1][1][1])/2];
    let ry = [(boxs_pos[2][0][1] + boxs_pos[2][1][1])/2, (boxs_pos[3][0][1] + boxs_pos[3][1][1])/2];
    let tx = [(boxs_pos[4][0][0] + boxs_pos[4][1][0])/2, (boxs_pos[5][0][0] + boxs_pos[5][1][0])/2];
    let bx = [(boxs_pos[6][0][0] + boxs_pos[6][1][0])/2, (boxs_pos[7][0][0] + boxs_pos[7][1][0])/2];

    let [ml, bl] = linear_equation(edges_pos[0], ly[0], edges_pos[1], ly[1]);
    let [mr, br] = linear_equation(edges_pos[2], ry[0], edges_pos[3], ly[1]);
    let [mt, bt] = linear_equation(tx[0], edges_pos[4], tx[1], edges_pos[5]);
    let [mb, bb] = linear_equation(bx[0], edges_pos[6], bx[1], edges_pos[7]);

    let [top_xl, top_yl] = find_intersection(ml, bl, mt, bt);
    let [top_xr, top_yr] = find_intersection(mr, br, mt, bt);
    let [bottom_xl, bottom_yl] = find_intersection(ml, bl, mb, bb);
    let [bottom_xr, bottom_yr] = find_intersection(mr, br, mb, bb);

    ([[top_xl, top_yl], [top_xr, top_yr], [bottom_xl, bottom_yl], [bottom_xr, bottom_yr]], [[ml, bl], [mr, br], [mt, bt], [mb, bb]])
}

pub fn length_line(points: [[i32; 2]; 4], mbs: [[f32; 2]; 4], xpoints: &Vec<i32>, ypoints: &Vec<i32>) -> (Vec<[[f32; 2]; 2]>, Vec<[[i32; 2]; 2]>, Vec<[String; 1]>) {
    // find length from linear line(m, b)
    // return most err length, middle lenght
    let mut results = vec![];
    let mut results_pos = vec![];
    let mut results_pos_text = vec![];
    let [[top_xl, top_yl], [top_xr, top_yr], [bottom_xl, bottom_yl], [bottom_xr, bottom_yr]] = points;
    let [[ml, bl], [mr, br], [mt, bt], [mb, bb]] = mbs;
    
    // left
    let max_left_t = ((top_yl as f32 - bl)/ml).round() as i32;
    let middle_left = ((ypoints[1] as f32 - bl)/ml).round() as i32;
    let max_left_b = ((bottom_yl as f32 - bl)/ml).round() as i32;
    let err_left_t = max_left_t - xpoints[0];
    let err_left_m = middle_left - xpoints[0];
    let err_left_b = max_left_b - xpoints[0];
    let left_length;
    let max_err;
    let middle_err = pixel2cm(ypoints, err_left_m);
    let middle_length = 9.0 - middle_err;
    if err_left_t.abs() > err_left_b.abs() {
        max_err = pixel2cm(ypoints, err_left_t);
        left_length = 9.0 - max_err;
        results_pos.push([[top_xl, top_yl], [middle_left, ypoints[1]]]);
        results_pos_text.push(["top-left".to_string()]);
    } else {
        max_err = pixel2cm(ypoints, err_left_b);
        left_length = 9.0 - max_err;
        results_pos.push([[bottom_xl, bottom_yl], [middle_left, ypoints[1]]]);
        results_pos_text.push(["bottom-left".to_string()]);
    }
    results.push([[left_length, -max_err], [middle_length, -middle_err]]);

    // right
    let max_right_t = ((top_yr as f32 - br)/mr).round() as i32;
    let middle_right = ((ypoints[1] as f32 - br)/mr).round() as i32;
    let max_right_b = ((bottom_yr as f32 - br)/mr).round() as i32;
    let err_right_t = xpoints[2] - max_right_t;
    let err_right_m = xpoints[2] - middle_right;
    let err_right_b = xpoints[2] - max_right_b;
    let right_length;
    let max_err;
    let middle_err = pixel2cm(ypoints, err_right_m);
    let middle_length = 9.0 - middle_err;
    if err_right_t.abs() > err_right_b.abs() {
        max_err = pixel2cm(ypoints, err_right_t);
        right_length = 9.0 - max_err;
        results_pos.push([[top_xr, top_yr], [middle_right, ypoints[1]]]);
        results_pos_text.push(["top-right".to_string()]);
    } else {
        max_err = pixel2cm(ypoints, err_right_b);
        right_length = 9.0 - max_err;
        results_pos.push([[bottom_xr, bottom_yr], [middle_right, ypoints[1]]]);
        results_pos_text.push(["bottom-right".to_string()]);
    }
    results.push([[right_length, -max_err], [middle_length, -middle_err]]);

    // top
    let max_top_l = ((mt * top_xl as f32) + bt).round() as i32;
    let middle_top = ((mt * xpoints[1] as f32) + bt).round() as i32;
    let max_top_r = ((mt * top_xr as f32) + bt).round() as i32;
    let err_top_l = max_top_l - ypoints[0];
    let err_top_m = middle_top - ypoints[0];
    let err_top_r = max_top_r - ypoints[0];
    let top_length;
    let max_err;
    let middle_err = pixel2cm(ypoints, err_top_m);
    let middle_length = 7.0 - middle_err;
    if err_top_l.abs() > err_top_r.abs() {
        max_err = pixel2cm(ypoints, err_top_l);
        top_length = 7.0 - max_err;
        results_pos.push([[top_xl, top_yl], [xpoints[1], middle_top]]);
        results_pos_text.push(["top-left".to_string()]);
    } else {
        max_err = pixel2cm(ypoints, err_top_r);
        top_length = 7.0 - max_err;
        results_pos.push([[top_xr, top_yr], [xpoints[1], middle_top]]);
        results_pos_text.push(["top-right".to_string()]);
    }
    results.push([[top_length, -max_err], [middle_length, -middle_err]]);

    // bottom
    let max_bottom_l = ((mb * bottom_xl as f32) + bb).round() as i32;
    let middle_bottom = ((mb * xpoints[1] as f32) + bb).round() as i32;
    let max_bottom_r = ((mb * bottom_xr as f32) + bb).round() as i32;
    let err_bottom_l = ypoints[2] - max_bottom_l;
    let err_bottom_m = ypoints[2] - middle_bottom;
    let err_bottom_r = ypoints[2] - max_bottom_r;
    let bottom_length;
    let max_err;
    let middle_err = pixel2cm(ypoints, err_bottom_m);
    let middle_length = 7.0 - middle_err;
    if err_bottom_l.abs() > err_bottom_r.abs() {
        max_err = pixel2cm(ypoints, err_bottom_l);
        bottom_length = 7.0 - max_err;
        results_pos.push([[bottom_xl, bottom_yl], [xpoints[1], middle_bottom]]);
        results_pos_text.push(["bottom-left".to_string()]);
    } else {
        max_err = pixel2cm(ypoints, err_bottom_r);
        bottom_length = 7.0 - max_err;
        results_pos.push([[bottom_xr, bottom_yr], [xpoints[1], middle_bottom]]);
        results_pos_text.push(["bottom-right".to_string()]);
    }
    results.push([[bottom_length, -max_err], [middle_length, -middle_err]]);

    (results, results_pos, results_pos_text)
}

pub fn distance_pixel(x1: usize, y1: usize, x2: usize, y2: usize) -> i32 {
    // find distance from 2 point return pixel distance
    ((x2 as f32 - x1 as f32).powi(2) + (y2 as f32 - y1 as f32).powi(2)).sqrt().round() as i32
}

pub fn calculate_angle(distance: f32) -> f32 { 
    distance*2.0
}