use std::collections::HashMap;

const DCS_FINALIZER: &str = "\x1b\\";

fn code_to_sixel(code: u8, repeat: u32, out: &mut String) {
    let c = (code + 63) as char;
    if repeat > 3 {
        out.push('!');
        out.push_str(&itoa(repeat));
        out.push(c);
    } else {
        for _ in 0..repeat {
            out.push(c);
        }
    }
}

#[inline(always)]
fn itoa(mut n: u32) -> String {
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    if n == 0 {
        return "0".to_string();
    }
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    String::from_utf8_lossy(&buf[i..]).to_string()
}

#[inline(always)]
fn itoa_writer(mut n: u32, out: &mut String) {
    if n == 0 {
        out.push('0');
        return;
    }
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    out.push_str(std::str::from_utf8(&buf[i..]).unwrap());
}

fn introducer(background_select: u8) -> String {
    if background_select == 0 {
        String::from("\x1bP0;0;q")
    } else {
        format!("\x1bP0;{};q", background_select)
    }
}

/// 3D color lookup table: 3 bits per channel = 512 entries.
/// Each entry stores the nearest palette index.
struct ColorLut {
    table: [u16; 512],
    palette: Vec<(u8, u8, u8)>,
}

impl ColorLut {
    fn new() -> Self {
        Self {
            table: [0; 512],
            palette: Vec::new(),
        }
    }

    fn rebuild(&mut self, palette: &[(u8, u8, u8)]) {
        self.palette.clear();
        self.palette.extend_from_slice(palette);

        if palette.is_empty() {
            return;
        }

        // For each of 512 LUT entries (3 bits R, 3 bits G, 3 bits B),
        // find the nearest palette color.
        for r3 in 0..8u8 {
            let r_mid = (r3 as u32 * 255 + 3) / 7;
            for g3 in 0..8u8 {
                let g_mid = (g3 as u32 * 255 + 3) / 7;
                for b3 in 0..8u8 {
                    let b_mid = (b3 as u32 * 255 + 3) / 7;
                    let key = ((r3 as usize) << 6) | ((g3 as usize) << 3) | (b3 as usize);
                    let mut best_dist = u32::MAX;
                    let mut best_idx = 0u16;
                    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
                        let dr = r_mid as i32 - pr as i32;
                        let dg = g_mid as i32 - pg as i32;
                        let db = b_mid as i32 - pb as i32;
                        let d = (dr * dr + dg * dg + db * db) as u32;
                        if d < best_dist {
                            best_dist = d;
                            best_idx = i as u16;
                        }
                    }
                    self.table[key] = best_idx;
                }
            }
        }
    }

    #[inline(always)]
    fn lookup(&self, r: u8, g: u8, b: u8) -> u16 {
        let r3 = r >> 5;
        let g3 = g >> 5;
        let b3 = b >> 5;
        let key = ((r3 as usize) << 6) | ((g3 as usize) << 3) | (b3 as usize);
        self.table[key]
    }
}

fn median_cut(pixels: &[(u8, u8, u8)], max_colors: usize) -> Vec<(u8, u8, u8)> {
    if pixels.is_empty() || max_colors == 0 {
        return Vec::new();
    }

    struct Box {
        indices: Vec<usize>,
        r_min: u8,
        r_max: u8,
        g_min: u8,
        g_max: u8,
        b_min: u8,
        b_max: u8,
    }

    fn calc_bounds(pixels: &[(u8, u8, u8)], indices: &[usize]) -> (u8, u8, u8, u8, u8, u8) {
        let (mut r_min, mut r_max) = (255u8, 0u8);
        let (mut g_min, mut g_max) = (255u8, 0u8);
        let (mut b_min, mut b_max) = (255u8, 0u8);
        for &idx in indices {
            let (r, g, b) = pixels[idx];
            if r < r_min {
                r_min = r;
            }
            if r > r_max {
                r_max = r;
            }
            if g < g_min {
                g_min = g;
            }
            if g > g_max {
                g_max = g;
            }
            if b < b_min {
                b_min = b;
            }
            if b > b_max {
                b_max = b;
            }
        }
        (r_min, r_max, g_min, g_max, b_min, b_max)
    }

    fn split_box(b: &mut Box, pixels: &[(u8, u8, u8)]) -> Option<(Box, Box)> {
        let dr = b.r_max - b.r_min;
        let dg = b.g_max - b.g_min;
        let db = b.b_max - b.b_min;

        if dr >= dg && dr >= db {
            b.indices.sort_by_key(|&i| pixels[i].0);
        } else if dg >= db {
            b.indices.sort_by_key(|&i| pixels[i].1);
        } else {
            b.indices.sort_by_key(|&i| pixels[i].2);
        }

        let mid = b.indices.len() / 2;
        if mid == 0 || mid == b.indices.len() {
            return None;
        }

        let right_indices = b.indices.split_off(mid);
        let left_indices = std::mem::take(&mut b.indices);

        let (lr_min, lr_max, lg_min, lg_max, lb_min, lb_max) = calc_bounds(pixels, &left_indices);
        let (rr_min, rr_max, rg_min, rg_max, rb_min, rb_max) = calc_bounds(pixels, &right_indices);

        Some((
            Box {
                indices: left_indices,
                r_min: lr_min,
                r_max: lr_max,
                g_min: lg_min,
                g_max: lg_max,
                b_min: lb_min,
                b_max: lb_max,
            },
            Box {
                indices: right_indices,
                r_min: rr_min,
                r_max: rr_max,
                g_min: rg_min,
                g_max: rg_max,
                b_min: rb_min,
                b_max: rb_max,
            },
        ))
    }

    let all_idx: Vec<usize> = (0..pixels.len()).collect();
    let (r_min, r_max, g_min, g_max, b_min, b_max) = calc_bounds(pixels, &all_idx);
    let mut boxes = vec![Box {
        indices: all_idx,
        r_min,
        r_max,
        g_min,
        g_max,
        b_min,
        b_max,
    }];

    while boxes.len() < max_colors {
        boxes.sort_by(|a, b| {
            let ra = (a.r_max - a.r_min)
                .max(a.g_max - a.g_min)
                .max(a.b_max - a.b_min);
            let rb = (b.r_max - b.r_min)
                .max(b.g_max - b.g_min)
                .max(b.b_max - b.b_min);
            rb.cmp(&ra)
        });
        let mut largest = boxes.remove(0);
        match split_box(&mut largest, pixels) {
            Some((left, right)) => {
                boxes.push(left);
                boxes.push(right);
            }
            None => {
                boxes.push(largest);
                break;
            }
        }
    }

    boxes
        .iter()
        .map(|b| {
            let n = b.indices.len() as u32;
            let (r_sum, g_sum, b_sum) =
                b.indices.iter().fold((0u32, 0u32, 0u32), |(r, g, b), &i| {
                    (
                        r + pixels[i].0 as u32,
                        g + pixels[i].1 as u32,
                        b + pixels[i].2 as u32,
                    )
                });
            ((r_sum / n) as u8, (g_sum / n) as u8, (b_sum / n) as u8)
        })
        .collect()
}

fn byte_stride(data: &[u8], n: usize) -> usize {
    let total = data.len();
    if total == n * 4 {
        return 4;
    }
    if total == n * 3 {
        return 3;
    }
    panic!(
        "data length {} does not match {} * 3 (RGB) or {} * 4 (RGBA)",
        total, n, n
    );
}

fn write_palette_defs(palette: &[(u8, u8, u8)], out: &mut String) {
    for (i, &(r, g, b)) in palette.iter().enumerate() {
        let rp = ((r as u32 * 100 + 127) / 255) as u8;
        let gp = ((g as u32 * 100 + 127) / 255) as u8;
        let bp = ((b as u32 * 100 + 127) / 255) as u8;
        out.push('#');
        itoa_writer(i as u32, out);
        out.push_str(";2;");
        itoa_writer(rp as u32, out);
        out.push(';');
        itoa_writer(gp as u32, out);
        out.push(';');
        itoa_writer(bp as u32, out);
    }
}

/// Build sixel data for a single 6-row band, writing directly to output.
/// `indices` are 0-based palette indices.
#[allow(clippy::too_many_arguments)]
fn emit_band_sixels(
    indices: &[u16],
    width: usize,
    band: usize,
    band_h: usize,
    n_colors: usize,
    out: &mut String,
    // Reusable buffers to avoid per-band allocation
    color_data: &mut Vec<String>,
    used_colors: &mut Vec<u16>,
    slot_map: &mut [i16],
    last_code: &mut [u8],
    last_code_init: &mut [bool],
    cur_code: &mut [u8],
    accu: &mut [u32],
) {
    color_data.clear();
    used_colors.clear();
    slot_map.iter_mut().for_each(|s| *s = -1);
    last_code.iter_mut().for_each(|s| *s = 0);
    last_code_init.iter_mut().for_each(|s| *s = false);
    cur_code.iter_mut().for_each(|s| *s = 0);
    accu.iter_mut().for_each(|s| *s = 1);

    for x in 0..width {
        for s in cur_code.iter_mut().take(color_data.len()) {
            *s = 0
        }

        for y in 0..band_h {
            let idx = indices[(band + y) * width + x] as usize + 1;
            if idx > n_colors {
                continue;
            }
            if slot_map[idx] == -1 {
                slot_map[idx] = color_data.len() as i16;
                color_data.push(String::new());
                used_colors.push(idx as u16);
                if x > 0 {
                    last_code[color_data.len() - 1] = 0;
                    last_code_init[color_data.len() - 1] = true;
                    accu[color_data.len() - 1] = x as u32;
                }
            }
            cur_code[slot_map[idx] as usize] |= 1 << y;
        }

        for s in 0..color_data.len() {
            if last_code_init[s] && cur_code[s] == last_code[s] {
                accu[s] += 1;
            } else {
                if last_code_init[s] && cur_code[s] != last_code[s] {
                    code_to_sixel(last_code[s], accu[s], &mut color_data[s]);
                }
                last_code[s] = cur_code[s];
                last_code_init[s] = true;
                accu[s] = 1;
            }
        }
    }

    for s in 0..color_data.len() {
        if last_code_init[s] && last_code[s] != 0 {
            code_to_sixel(last_code[s], accu[s], &mut color_data[s]);
        }
    }

    // Emit to output directly
    for (s, data) in color_data.iter().enumerate() {
        if data.is_empty() {
            continue;
        }
        let idx = used_colors[s];
        out.push('#');
        itoa_writer(idx as u32 - 1, out);
        out.push_str(data);
        out.push('$');
    }
}

/// Encode RGB or RGBA pixel data to a SIXEL DCS string.
///
/// `data` can be RGB888 (3 bytes/pixel) or RGBA (4 bytes/pixel).
/// `background_select`: 0=transparent (default), 1=opaque background.
pub fn image2sixel(
    data: &[u8],
    width: usize,
    height: usize,
    max_colors: u8,
    background_select: u8,
) -> String {
    if width == 0 || height == 0 || data.is_empty() {
        return String::new();
    }

    let n = width * height;
    let max_colors = max_colors as usize;
    let stride = byte_stride(data, n);

    let mut unique_colors: HashMap<u32, usize> = HashMap::new();
    let mut palette: Vec<(u8, u8, u8)> = Vec::new();
    let mut indices = vec![0u16; n];

    for (i, item) in indices.iter_mut().enumerate().take(n) {
        let pi = i * stride;
        let r = data[pi];
        let g = data[pi + 1];
        let b = data[pi + 2];
        let key = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        let idx = *unique_colors.entry(key).or_insert_with(|| {
            let idx = palette.len();
            palette.push((r, g, b));
            idx
        });
        *item = idx as u16;
    }

    if palette.len() > max_colors {
        let pixels: Vec<(u8, u8, u8)> = (0..n)
            .map(|i| {
                let pi = i * stride;
                (data[pi], data[pi + 1], data[pi + 2])
            })
            .collect();
        palette = median_cut(&pixels, max_colors);
        for (i, item) in indices.iter_mut().enumerate().take(n) {
            let pi = i * stride;
            let (r, g, b) = (data[pi], data[pi + 1], data[pi + 2]);
            let mut best_dist = u32::MAX;
            let mut best_idx = 0usize;
            for (j, &(pr, pg, pb)) in palette.iter().enumerate() {
                let dr = r as i32 - pr as i32;
                let dg = g as i32 - pg as i32;
                let db = b as i32 - pb as i32;
                let d = (dr * dr + dg * dg + db * db) as u32;
                if d < best_dist {
                    best_dist = d;
                    best_idx = j;
                }
            }
            // indices[i] = best_idx as u16;
            *item = best_idx as u16;
        }
    }

    let n_colors = palette.len();

    let capacity = width * height * 2 + 256;
    let mut out = String::with_capacity(capacity);

    out.push_str(&introducer(background_select));
    out.push('"');
    itoa_writer(1, &mut out);
    out.push(';');
    itoa_writer(1, &mut out);
    out.push(';');
    itoa_writer(width as u32, &mut out);
    out.push(';');
    itoa_writer(height as u32, &mut out);
    write_palette_defs(&palette, &mut out);

    // Reusable buffers for band emission
    let mut color_data: Vec<String> = Vec::new();
    let mut used_colors: Vec<u16> = Vec::new();
    let mut slot_map: Vec<i16> = vec![-1; n_colors + 1];
    let mut last_code: Vec<u8> = vec![0; n_colors + 1];
    let mut last_code_init: Vec<bool> = vec![false; n_colors + 1];
    let mut cur_code: Vec<u8> = vec![0; n_colors + 1];
    let mut accu: Vec<u32> = vec![1; n_colors + 1];

    for band in (0..height).step_by(6) {
        let band_h = std::cmp::min(6, height - band);
        emit_band_sixels(
            &indices,
            width,
            band,
            band_h,
            n_colors,
            &mut out,
            &mut color_data,
            &mut used_colors,
            &mut slot_map,
            &mut last_code,
            &mut last_code_init,
            &mut cur_code,
            &mut accu,
        );

        if band + 6 < height {
            out.push_str("-\n");
        }
    }

    out.push_str(DCS_FINALIZER);
    out
}

/// Encode RGBA pixel data using a pre-built palette, skipping quantization.
///
/// `data` must be RGBA (4 bytes/pixel). Transparent pixels (alpha=0) are skipped.
/// `palette` is a list of RGB colors.
pub fn sixel_encode(data: &[u8], width: usize, height: usize, palette: &[(u8, u8, u8)]) -> String {
    if width == 0 || height == 0 || data.is_empty() {
        return String::new();
    }

    let n = width * height;
    let stride = byte_stride(data, n);

    let mut palette_packed: Vec<u32> = Vec::with_capacity(palette.len());
    for &(r, g, b) in palette {
        palette_packed.push(((r as u32) << 16) | ((g as u32) << 8) | b as u32);
    }

    let mut indices = vec![0u16; n];
    for (i, item) in indices.iter_mut().enumerate().take(n) {
        let pi = i * stride;
        let a = if stride == 4 { data[pi + 3] } else { 0xff };
        if a == 0 {
            continue;
        }
        let key = ((data[pi] as u32) << 16) | ((data[pi + 1] as u32) << 8) | data[pi + 2] as u32;
        let idx = palette_packed.iter().position(|&c| c == key).unwrap_or(0);
        // indices[i] = idx as u16 + 1;
        *item = idx as u16 + 1;
    }

    let n_colors = palette.len();

    let capacity = width * height * 2 + 256;
    let mut out = String::with_capacity(capacity);

    out.push_str(&introducer(0));
    out.push('"');
    itoa_writer(1, &mut out);
    out.push(';');
    itoa_writer(1, &mut out);
    out.push(';');
    itoa_writer(width as u32, &mut out);
    out.push(';');
    itoa_writer(height as u32, &mut out);
    write_palette_defs(palette, &mut out);

    let mut color_data: Vec<String> = Vec::new();
    let mut used_colors: Vec<u16> = Vec::new();
    let mut slot_map: Vec<i16> = vec![-1; n_colors + 1];
    let mut last_code: Vec<u8> = vec![0; n_colors + 1];
    let mut last_code_init: Vec<bool> = vec![false; n_colors + 1];
    let mut cur_code: Vec<u8> = vec![0; n_colors + 1];
    let mut accu: Vec<u32> = vec![1; n_colors + 1];

    for band in (0..height).step_by(6) {
        let band_h = std::cmp::min(6, height - band);
        emit_band_sixels(
            &indices,
            width,
            band,
            band_h,
            n_colors,
            &mut out,
            &mut color_data,
            &mut used_colors,
            &mut slot_map,
            &mut last_code,
            &mut last_code_init,
            &mut cur_code,
            &mut accu,
        );

        if band + 6 < height {
            out.push_str("-\n");
        }
    }

    out.push_str(DCS_FINALIZER);
    out
}

pub struct SixelEncoder {
    max_colors: u8,
    background_select: u8,
    color_lut: ColorLut,
    // Reusable buffers across frames
    palette: Vec<(u8, u8, u8)>,
    indices: Vec<u16>,
    out: String,
    color_data: Vec<String>,
    used_colors: Vec<u16>,
    slot_map: Vec<i16>,
    last_code: Vec<u8>,
    last_code_init: Vec<bool>,
    cur_code: Vec<u8>,
    accu: Vec<u32>,
    cached_palette_len: usize,
}

impl SixelEncoder {
    pub fn new(max_colors: u8) -> Self {
        Self {
            max_colors,
            background_select: 0,
            color_lut: ColorLut::new(),
            palette: Vec::new(),
            indices: Vec::new(),
            out: String::new(),
            color_data: Vec::new(),
            used_colors: Vec::new(),
            slot_map: Vec::new(),
            last_code: Vec::new(),
            last_code_init: Vec::new(),
            cur_code: Vec::new(),
            accu: Vec::new(),
            cached_palette_len: 0,
        }
    }

    pub fn with_background_select(max_colors: u8, background_select: u8) -> Self {
        Self {
            max_colors,
            background_select,
            color_lut: ColorLut::new(),
            palette: Vec::new(),
            indices: Vec::new(),
            out: String::new(),
            color_data: Vec::new(),
            used_colors: Vec::new(),
            slot_map: Vec::new(),
            last_code: Vec::new(),
            last_code_init: Vec::new(),
            cur_code: Vec::new(),
            accu: Vec::new(),
            cached_palette_len: 0,
        }
    }

    pub fn encode_frame(&mut self, width: usize, height: usize, data: &[u8]) -> String {
        if width == 0 || height == 0 || data.is_empty() {
            return String::new();
        }

        let n = width * height;
        let max_colors = self.max_colors as usize;
        let stride = byte_stride(data, n);

        // collect unique colors
        self.palette.clear();
        {
            let mut unique_colors: HashMap<u32, usize> = HashMap::with_capacity(1024);
            self.indices.resize(n, 0);

            for i in 0..n {
                let pi = i * stride;
                let r = data[pi];
                let g = data[pi + 1];
                let b = data[pi + 2];
                let key = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
                let idx = *unique_colors.entry(key).or_insert_with(|| {
                    let idx = self.palette.len();
                    self.palette.push((r, g, b));
                    idx
                });
                self.indices[i] = idx as u16;
            }
        }

        // quantize if needed
        if self.palette.len() > max_colors {
            // Build pixel list for median cut
            let pixels: Vec<(u8, u8, u8)> = (0..n)
                .map(|i| {
                    let pi = i * stride;
                    (data[pi], data[pi + 1], data[pi + 2])
                })
                .collect();
            self.palette = median_cut(&pixels, max_colors);
            self.color_lut.rebuild(&self.palette);

            for i in 0..n {
                let pi = i * stride;
                self.indices[i] = self.color_lut.lookup(data[pi], data[pi + 1], data[pi + 2]);
            }
        } else if self.palette.len() != self.cached_palette_len {
            self.color_lut.rebuild(&self.palette);
            self.cached_palette_len = self.palette.len();
        }

        let n_colors = self.palette.len();

        // encode sixel output
        self.out.clear();
        let capacity = width * height * 2 + 256;
        self.out
            .reserve(capacity.saturating_sub(self.out.capacity()));

        self.out.push_str(&introducer(self.background_select));
        self.out.push('"');
        itoa_writer(1, &mut self.out);
        self.out.push(';');
        itoa_writer(1, &mut self.out);
        self.out.push(';');
        itoa_writer(width as u32, &mut self.out);
        self.out.push(';');
        itoa_writer(height as u32, &mut self.out);
        write_palette_defs(&self.palette, &mut self.out);

        // Reusable buffers for band emission
        self.slot_map.resize(n_colors + 1, -1);
        self.last_code.resize(n_colors + 1, 0);
        self.last_code_init.resize(n_colors + 1, false);
        self.cur_code.resize(n_colors + 1, 0);
        self.accu.resize(n_colors + 1, 1);

        for band in (0..height).step_by(6) {
            let band_h = std::cmp::min(6, height - band);
            emit_band_sixels(
                &self.indices,
                width,
                band,
                band_h,
                n_colors,
                &mut self.out,
                &mut self.color_data,
                &mut self.used_colors,
                &mut self.slot_map,
                &mut self.last_code,
                &mut self.last_code_init,
                &mut self.cur_code,
                &mut self.accu,
            );

            if band + 6 < height {
                self.out.push_str("-\n");
            }
        }

        self.out.push_str(DCS_FINALIZER);
        std::mem::take(&mut self.out)
    }
}
