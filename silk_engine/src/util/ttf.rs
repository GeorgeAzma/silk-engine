use crate::{RES_PATH, util::ReaderBe};

#[derive(Default, Debug, Clone)]
pub(crate) struct GlyphMetrics {
    pub(crate) xmin: i16,
    pub(crate) ymin: i16,
    pub(crate) xmax: i16,
    pub(crate) ymax: i16,
    pub(crate) advance_width: u16,
}

impl GlyphMetrics {
    pub(crate) fn width(&self) -> u16 {
        (self.xmax - self.xmin) as u16
    }

    pub(crate) fn height(&self) -> u16 {
        (self.ymax - self.ymin) as u16
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct GlyphData {
    pub(crate) metric: GlyphMetrics,
    pub(crate) points: Vec<(i16, i16, bool)>, // x, y, on_curve
    pub(crate) contour_end_idxs: Vec<u16>,
}

#[derive(Default, Debug, Clone)]
struct TableOffs {
    maxp: u32,
    head: u32,
    loca: u32,
    glyf: u32,
    cmap: u32,
    hhea: u32,
    hmtx: u32,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct Head {
    pub(crate) num_glyphs: u16,
    #[allow(unused)] // TODO: use for text layout
    em_units: u16,
    pub(crate) glob_xmin: i16,
    pub(crate) glob_ymin: i16,
    pub(crate) glob_xmax: i16,
    pub(crate) glob_ymax: i16,
    _lowest_rec_ppem: u16, // smallest readable px size
    loc_bytes: u16,
}

impl Head {
    pub(crate) fn max_width(&self) -> u16 {
        (self.glob_xmax - self.glob_xmin) as u16
    }

    pub(crate) fn max_height(&self) -> u16 {
        (self.glob_ymax - self.glob_ymin) as u16
    }
}

pub(crate) struct Ttf {
    pub(crate) head: Head,
    pub(crate) glyphs: Vec<GlyphData>,
    pub(crate) idx2uni: Vec<char>,
}

// TTF parsing: https://youtu.be/SO83KQuuZvg
impl Ttf {
    pub(crate) fn new(name: &str) -> Self {
        let path = format!("{RES_PATH}/fonts/{name}.ttf");
        let bytes = std::fs::read(path).unwrap();

        let mut reader = ReaderBe::new(&bytes);
        let table_offs = Self::read_table_offs(&mut reader);
        let head = Self::read_head(&mut reader, &table_offs);
        let glyph_offs = Self::read_glyph_offs(
            &mut reader,
            table_offs.loca,
            table_offs.glyf,
            head.num_glyphs,
            head.loc_bytes,
        );
        let idx2uni = Self::read_idx2uni_mappings(&mut reader, table_offs.cmap);
        let glyphs = Self::read_glyphs(&mut reader, &glyph_offs, &table_offs);
        Self {
            head,
            glyphs,
            idx2uni,
        }
    }

    fn read_table_offs(reader: &mut ReaderBe) -> TableOffs {
        reader.skip(4); // scalar_type
        let num_tables = reader.read16();
        reader.skip(6); // search range, entry selector, range shift

        let mut table_offs = TableOffs::default();

        for _ in 0..num_tables {
            let tag = reader.read32();
            let _check_sum = reader.read32();
            let off = reader.read32();
            let _len = reader.read32();
            match &tag.to_be_bytes() {
                b"loca" => table_offs.loca = off,
                b"maxp" => table_offs.maxp = off,
                b"head" => table_offs.head = off,
                b"glyf" => table_offs.glyf = off,
                b"cmap" => table_offs.cmap = off,
                b"hhea" => table_offs.hhea = off,
                b"hmtx" => table_offs.hmtx = off,
                _ => {}
            }
        }
        table_offs
    }

    fn read_head(reader: &mut ReaderBe, table_offs: &TableOffs) -> Head {
        reader.goto(table_offs.maxp as usize);
        reader.skip(4); // version
        let num_glyphs = reader.read16();

        reader.goto(table_offs.head as usize);
        // version, font revision, check sum adjustment, magic num, flags,
        reader.skip(18);
        let em_units = reader.read16(); // (64 to 16384)
        reader.skip(16); // created date, modified date
        let glob_xmin = reader.read16() as i16;
        let glob_ymin = reader.read16() as i16;
        let glob_xmax = reader.read16() as i16;
        let glob_ymax = reader.read16() as i16;
        reader.skip(2); // mac style
        let lowest_rec_ppem = reader.read16(); // dir hint flag
        reader.skip(2); // font dir hint (deprecated)
        let loc_bytes = if reader.read16() == 0 { 2 } else { 4 };
        reader.skip(2); // glyph data format

        Head {
            num_glyphs,
            em_units,
            glob_xmin,
            glob_ymin,
            glob_xmax,
            glob_ymax,
            _lowest_rec_ppem: lowest_rec_ppem,
            loc_bytes,
        }
    }

    fn read_hmetrics(
        reader: &mut ReaderBe,
        glyph_offs: &[u32],
        table_offs: &TableOffs,
    ) -> Vec<u16> {
        reader.goto(table_offs.hhea as usize);
        // major/minor version, ascent, descent, line gap, max advance width
        // min left/right side bearing, xmax extent, caret slope rise/run
        // caret off, reserved64, metric data format (note: all 16 bits)
        reader.skip(34);
        let num_hmetrics = reader.read16() as usize;

        // read glyph advance widths
        reader.goto(table_offs.hmtx as usize);
        let mut glyph_advance_widths = (0..glyph_offs.len())
            .map(|_| {
                let advance_width = reader.read16();
                let _left_side_bearing = reader.read16() as i16;
                advance_width
            })
            .collect::<Vec<_>>();
        // some fonts have mono-spaced glyphs at the end
        // which have same advance width as last
        for i in num_hmetrics..glyph_offs.len() {
            glyph_advance_widths[i] = glyph_advance_widths[num_hmetrics - 1];
        }
        glyph_advance_widths
    }

    fn read_glyphs(
        reader: &mut ReaderBe,
        glyph_offs: &[u32],
        table_offs: &TableOffs,
    ) -> Vec<GlyphData> {
        let glyph_advance_widths = Self::read_hmetrics(reader, glyph_offs, table_offs);
        let mut glyphs = vec![GlyphData::default(); glyph_offs.len()];
        for i in 0..glyph_offs.len() {
            glyphs[i] = Self::read_glyph(reader, glyph_offs, i as u16);
            glyphs[i].metric.advance_width = glyph_advance_widths[i];
        }
        glyphs
    }

    fn read_glyph(reader: &mut ReaderBe, glyph_offs: &[u32], glyph_idx: u16) -> GlyphData {
        let glyph_off = glyph_offs[glyph_idx as usize];
        reader.goto(glyph_off as usize);
        let contour_count = reader.read16() as i16;
        let is_simple = contour_count >= 0; // not compound
        if is_simple {
            Self::read_simple_glyph(reader, contour_count)
        } else {
            Self::read_compound_glyph(reader, glyph_offs)
        }
    }

    fn read_glyph_offs(
        reader: &mut ReaderBe,
        loca_off: u32,
        glyf_off: u32,
        num_glyphs: u16,
        loc_bytes: u16,
    ) -> Vec<u32> {
        (0..num_glyphs as usize)
            .map(|i| {
                reader.goto(loca_off as usize + i * loc_bytes as usize);
                let glyph_off = if loc_bytes == 2 {
                    reader.read16() as u32 * 2
                } else {
                    reader.read32()
                };
                glyf_off + glyph_off
            })
            .collect()
    }

    fn read_simple_glyph(reader: &mut ReaderBe, contour_count: i16) -> GlyphData {
        assert!(
            contour_count >= 0,
            "expected simple glyph, got compound instead"
        );
        let xmin = reader.read16() as i16;
        let ymin = reader.read16() as i16;
        let xmax = reader.read16() as i16;
        let ymax = reader.read16() as i16;

        let mut num_points = 0;
        let mut contour_end_idxs = vec![0; contour_count as usize];
        for cei in contour_end_idxs.iter_mut() {
            *cei = reader.read16();
            num_points = num_points.max(*cei + 1);
        }

        let num_instrs = reader.read16() as usize;
        reader.skip(num_instrs); // skip instructions

        let mut flags = vec![0; num_points as usize];
        let mut i: usize = 0;
        while i < num_points as usize {
            let flag = reader.read8();
            flags[i] = flag;
            let repeat = ((flag >> 3) & 1) == 1;
            if repeat {
                let reps = reader.read8();
                for _ in 0..reps {
                    i += 1;
                    flags[i] = flag;
                }
            }
            i += 1;
        }

        let points = Self::read_coords(reader, &flags);
        GlyphData {
            metric: GlyphMetrics {
                xmin,
                ymin,
                xmax,
                ymax,
                advance_width: 0,
            },
            points,
            contour_end_idxs,
        }
    }

    fn read_compound_glyph(reader: &mut ReaderBe, glyph_offs: &[u32]) -> GlyphData {
        let xmin = reader.read16() as i16;
        let ymin = reader.read16() as i16;
        let xmax = reader.read16() as i16;
        let ymax = reader.read16() as i16;

        let mut points = vec![];
        let mut contour_end_idxs = vec![];
        loop {
            let (comp_glyph, is_last) = Self::read_component_glyph(reader, glyph_offs);
            contour_end_idxs.extend(
                comp_glyph
                    .contour_end_idxs
                    .into_iter()
                    .map(|end_idx| end_idx + points.len() as u16),
            );
            points.extend(comp_glyph.points);
            if is_last {
                break;
            }
        }

        GlyphData {
            metric: GlyphMetrics {
                xmin,
                ymin,
                xmax,
                ymax,
                advance_width: 0,
            },
            points,
            contour_end_idxs,
        }
    }

    fn read_component_glyph(reader: &mut ReaderBe, glyph_offs: &[u32]) -> (GlyphData, bool) {
        let flag = reader.read16();
        let glyph_idx = reader.read16();
        let _comp_glyph_off = glyph_offs[glyph_idx as usize];
        let args_2b = (flag & 1) == 1;
        let args_xy = ((flag >> 1) & 1) == 1;
        let _round_xy = ((flag >> 2) & 1) == 1;
        let single_scale = ((flag >> 3) & 1) == 1;
        let more_comps = ((flag >> 5) & 1) == 1;
        let xy_scale = ((flag >> 6) & 1) == 1;
        let mat2x2 = ((flag >> 7) & 1) == 1;
        let _has_instrs = ((flag >> 8) & 1) == 1;
        let _use_this_comp_metrics = ((flag >> 9) & 1) == 1;
        let _comps_overlap = ((flag >> 10) & 1) == 1;

        let (arg1, arg2) = if args_2b {
            (reader.read16() as i16, reader.read16() as i16)
        } else {
            (reader.read8() as i8 as i16, reader.read8() as i8 as i16)
        };
        assert!(
            args_xy,
            "TODO: args1&2 are point idx to be matched, not offsets"
        );
        let off_x = arg1;
        let off_y = arg2;

        let mut ihat_x = 1.0;
        let mut ihat_y = 0.0;
        let mut jhat_x = 0.0;
        let mut jhat_y = 1.0;
        let f2d14 = |u: u16| (u as i16) as f32 / (1 << 14) as f32;
        if single_scale {
            ihat_x = f2d14(reader.read16());
            jhat_y = ihat_x;
        } else if xy_scale {
            ihat_x = f2d14(reader.read16());
            ihat_y = f2d14(reader.read16());
        } else if mat2x2 {
            ihat_x = f2d14(reader.read16());
            ihat_y = f2d14(reader.read16());
            jhat_x = f2d14(reader.read16());
            jhat_y = f2d14(reader.read16());
        }
        let cur_comp_glyph_off = reader.idx();
        let mut simple_glyph = Self::read_glyph(reader, glyph_offs, glyph_idx);
        reader.goto(cur_comp_glyph_off);
        for (x, y, _) in simple_glyph.points.iter_mut() {
            let (xx, yy) = (*x, *y);
            let nx = (ihat_x * xx as f32 + jhat_x * yy as f32 + off_x as f32).round() as i32;
            let ny = (ihat_y * xx as f32 + jhat_y * yy as f32 + off_y as f32).round() as i32;
            assert!(
                nx <= i16::MAX as i32
                    && ny <= i16::MAX as i32
                    && nx >= i16::MIN as i32
                    && ny >= i16::MIN as i32,
                "expected transformed glyph point to be within i16 range"
            );
            *x = nx as i16;
            *y = ny as i16;
        }
        (simple_glyph, !more_comps)
    }

    fn read_coords(reader: &mut ReaderBe, flags: &[u8]) -> Vec<(i16, i16, bool)> {
        let mut points = vec![(0, 0, false); flags.len()];
        let mut read_coords = |points: &mut Vec<(i16, i16, bool)>, reading_x: bool| {
            let size_flag_off = if reading_x { 1 } else { 2 };
            let sign_or_skip_off = if reading_x { 4 } else { 5 };
            let mut point_val = 0;
            for i in 0..points.len() {
                let flag = flags[i];
                let on_curve = (flag & 1) == 1;
                let sign_or_skip = ((flag >> sign_or_skip_off) & 1) == 1;
                let size_flag = (flag >> size_flag_off) & 1;
                if size_flag == 1 {
                    let off = reader.read8();
                    let sign = if sign_or_skip { 1 } else { -1 };
                    point_val += off as i16 * sign;
                } else if !sign_or_skip {
                    point_val += reader.read16() as i16;
                }
                if reading_x {
                    points[i].0 = point_val;
                } else {
                    points[i].1 = point_val;
                }
                points[i].2 = on_curve;
            }
        };
        read_coords(&mut points, true);
        read_coords(&mut points, false);
        points
    }

    /// returns glyph index to unicode array
    fn read_idx2uni_mappings(reader: &mut ReaderBe, cmap_off: u32) -> Vec<char> {
        let mut idx2uni = vec!['\0'; 65536];
        reader.goto(cmap_off as usize);
        let _version = reader.read16();
        let subtables = reader.read16();
        let mut cmap_subtable_off = 0;
        let mut selected_unicode_ver_id = u16::MAX;
        for _ in 0..subtables {
            let platform_id = reader.read16();
            let platform_specific_id = reader.read16();
            let off = reader.read32();
            // unicode encoding
            if platform_id == 0 {
                if (platform_specific_id == 0
                    || platform_specific_id == 1
                    || platform_specific_id == 3
                    || platform_specific_id == 4)
                    && platform_specific_id > selected_unicode_ver_id
                {
                    cmap_subtable_off = off;
                    selected_unicode_ver_id = platform_specific_id;
                }
            }
            // microsoft encoding
            else if platform_id == 3
                && selected_unicode_ver_id == u16::MAX
                && (platform_specific_id == 1 || platform_specific_id == 10)
            {
                cmap_subtable_off = off;
            }
        }
        assert_ne!(
            cmap_subtable_off, 0,
            "font does not contain supported char map type"
        );
        reader.goto(cmap_off as usize + cmap_subtable_off as usize);
        let format = reader.read16();
        let mut has_read_missing_char_glyph = false;
        assert!(
            format == 12 || format == 4,
            "unsupported font cmap format: {format}"
        );
        if format == 4 {
            let _len = reader.read16();
            let _lang_code = reader.read16();
            let seg_count = reader.read16() / 2;
            reader.skip(6); // search range, entry selector, range shift
            let end_codes = (0..seg_count).map(|_| reader.read16()).collect::<Vec<_>>();
            reader.skip(2);
            let start_codes = (0..seg_count).map(|_| reader.read16()).collect::<Vec<_>>();
            let id_deltas = (0..seg_count).map(|_| reader.read16()).collect::<Vec<_>>();
            let id_range_offs = (0..seg_count)
                .map(|_| (reader.read16(), reader.idx()))
                .collect::<Vec<_>>();
            for i in 0..start_codes.len() {
                let end_code = end_codes[i];
                let mut cur_code = start_codes[i];
                if cur_code == 65535 {
                    break;
                }
                while cur_code <= end_code {
                    let mut glyph_idx;
                    if id_range_offs[i].0 == 0 {
                        glyph_idx = cur_code.wrapping_add(id_deltas[i]);
                    } else {
                        let old_reader_off = reader.idx();
                        let range_off_loc = id_range_offs[i].1 + id_range_offs[i].0 as usize;
                        let glyph_idx_arr_loc =
                            2 * (cur_code - start_codes[i]) as usize + range_off_loc;
                        reader.goto(glyph_idx_arr_loc);
                        glyph_idx = reader.read16();
                        if glyph_idx != 0 {
                            glyph_idx = glyph_idx.wrapping_add(id_deltas[i]);
                        }
                        reader.goto(old_reader_off);
                    }
                    // ornate parentheses have same glyph_idx as ascii parens
                    // because font may not support ornate paren rendering and renders it as ascii paren
                    // because of this it overwrote ascii paren's unicode
                    // so I added this if check to prevent overwrite
                    if idx2uni[glyph_idx as usize] == '\0' {
                        idx2uni[glyph_idx as usize] = char::from_u32(cur_code as u32).unwrap();
                    }
                    has_read_missing_char_glyph |= glyph_idx == 0;
                    cur_code += 1;
                }
            }
        } else if format == 12 {
            reader.skip(10); // reserved, subtable byte length including header, lang code
            let num_groups = reader.read32();
            for _ in 0..num_groups {
                let start_char_code = reader.read32();
                let end_char_code = reader.read32();
                let start_glyph_idx = reader.read32();
                let num_chars = end_char_code - start_char_code + 1;
                for char_code_off in 0..num_chars {
                    let char_code = start_char_code + char_code_off;
                    let glyph_idx = start_glyph_idx + char_code_off;
                    if idx2uni[glyph_idx as usize] == '\0' {
                        idx2uni[glyph_idx as usize] = char::from_u32(char_code).unwrap();
                    }
                    has_read_missing_char_glyph |= glyph_idx == 0;
                }
            }
        }
        if !has_read_missing_char_glyph {
            idx2uni[0] = '\u{65535}';
        }
        // trim useless '\0'
        if let Some(last) = idx2uni.iter().rposition(|x| *x != '\0') {
            idx2uni[..last + 1].to_owned()
        } else {
            idx2uni
        }
    }
}
