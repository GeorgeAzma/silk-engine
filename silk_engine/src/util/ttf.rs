use std::collections::HashMap;

use crate::{util::reader::ReaderBe, warn};

#[derive(Default, Debug, Clone)]
pub(crate) struct GlyphMetrics {
    pub(crate) xmin: i16,
    pub(crate) ymin: i16,
    pub(crate) xmax: i16,
    pub(crate) ymax: i16,
    pub(crate) advance_width: i16,
    pub(crate) left_side_bearing: i16,
    pub(crate) advance_height: i16,
    pub(crate) top_side_bearing: i16,
}

impl GlyphMetrics {
    pub(crate) fn width(&self) -> i16 {
        self.xmax - self.xmin
    }

    pub(crate) fn height(&self) -> i16 {
        self.ymax - self.ymin
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct GlyphData {
    pub(crate) metric: GlyphMetrics,
    pub(crate) points: Vec<(i16, i16, bool)>, // x, y, on_curve
    pub(crate) contour_end_idxs: Vec<u16>,
    pub(crate) index: u16,
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
    vhea: u32,
    vmtx: u32,
    kern: u32,
    gpos: u32,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct Head {
    pub(crate) num_glyphs: u16,
    pub(crate) em_units: u16,
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

struct Hmetrics {
    ascent: i16,
    descent: i16,
    line_gap: i16,
    glyph_hmetrics: Vec<(i16, i16)>, // advance width, left side bearing
}

struct Vmetrics {
    ascent: i16,
    descent: i16,
    glyph_vmetrics: Vec<(i16, i16)>, // advance height, top side bearing
}

pub(crate) struct Ttf {
    pub(crate) head: Head,
    pub(crate) ascent: i16,
    pub(crate) descent: i16,
    #[allow(unused)]
    pub(crate) vert_ascent: i16,
    #[allow(unused)]
    pub(crate) vert_descent: i16,
    pub(crate) line_gap: i16,
    /// HashMap<(first_glyph << 16 | second_glyph), kern_value_em>
    pub(crate) kernings: HashMap<u32, i16>,
    pub(crate) glyphs: Vec<GlyphData>,
    pub(crate) idx2uni: Vec<char>,
}

// TTF parsing: https://youtu.be/SO83KQuuZvg
impl Ttf {
    pub(crate) fn new(name: &str) -> Self {
        let path = format!("res/fonts/{name}.ttf");
        let bytes = std::fs::read(path).unwrap();

        let mut reader = ReaderBe::new(&bytes);
        let table_offs = Self::read_table_offs(&mut reader);
        let head = Self::read_head(&mut reader, &table_offs);
        let glyph_offs =
            Self::read_glyph_offs(&mut reader, &table_offs, head.num_glyphs, head.loc_bytes);
        let idx2uni = Self::read_idx2uni_mappings(&mut reader, table_offs.cmap, head.num_glyphs);
        let hmetrics = Self::read_hmetrics(&mut reader, &glyph_offs, &table_offs);
        let vmetrics = Self::read_vmetrics(&mut reader, &glyph_offs, &table_offs);
        let glyphs = Self::read_glyphs(
            &mut reader,
            &glyph_offs,
            &hmetrics.glyph_hmetrics,
            &vmetrics.glyph_vmetrics,
            &idx2uni,
        );
        let kernings = Self::read_kernings(&mut reader, &table_offs);
        Self {
            head,
            ascent: hmetrics.ascent,
            descent: hmetrics.descent,
            vert_ascent: vmetrics.ascent,
            vert_descent: vmetrics.descent,
            line_gap: hmetrics.line_gap,
            glyphs,
            idx2uni,
            kernings,
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
                b"vhea" => table_offs.vhea = off,
                b"vmtx" => table_offs.vmtx = off,
                b"kern" => table_offs.kern = off,
                b"GPOS" => table_offs.gpos = off,
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

    fn read_kernings(reader: &mut ReaderBe, table_offs: &TableOffs) -> HashMap<u32, i16> {
        let mut kernings = HashMap::new();
        if table_offs.kern == 0 {
            kernings.extend(Self::read_gpos(reader, table_offs));
            return kernings;
        }
        reader.goto(table_offs.kern as usize);
        // note: apple might be using 32 bits for version/num_tables
        let version = reader.read16();
        assert_eq!(0, version, "only OpenType fonts are supported");
        let num_tables = reader.read16();
        assert_ne!(num_tables, 0, "num kerning tables can't be 0");
        for _ in 0..num_tables {
            reader.skip(4); // version, length
            let coverage = reader.read16();
            let format = coverage & 0x00FF;
            match format {
                0 /* kerning pair ordered list */ => {
                    let num_pairs = reader.read16();
                    crate::info!("using format 0 with {num_pairs} pairs");
                    reader.skip(6); // search range, entry selector, range shift
                    for _ in 0..num_pairs {
                        let first_glyph = reader.read16();
                        let second_glyph = reader.read16();
                        let value = reader.read16() as i16;
                        kernings.insert(((first_glyph as u32) << 16) | second_glyph as u32, value);
                    }
                }
                1 => {}
                // kern format 2 is less common and maybe unsupported by windows
                2 /* kern_vals[][] + class subtable */ => {
                    todo!("kern format 2 is less common and maybe unsupported by windows")
                }
                // 3 is compacted 2, it is not as common/supported as others, so skip
                _ => warn!("invalid kern format: {format}, skipping kernings"),
            }
        }
        if kernings.is_empty() {
            kernings.extend(Self::read_gpos(reader, table_offs));
        }
        kernings
    }

    fn read_gpos(reader: &mut ReaderBe, table_offs: &TableOffs) -> HashMap<u32, i16> {
        fn read_feature_list(reader: &mut ReaderBe) -> HashMap<u32, Vec<u16>> {
            let idx = reader.idx();
            let mut features = HashMap::new();
            let count = reader.read16();
            for _ in 0..count {
                let tag = reader.read32();
                let off = reader.read16();
                let ret = reader.idx();
                let feature_idx = idx + off as usize;
                reader.goto(feature_idx);
                let _params = reader.read16();
                let lookup_count = reader.read16();
                let lookup_idxs = reader.read_arr16(lookup_count as usize);
                features.insert(tag, lookup_idxs);
                reader.goto(ret);
            }
            features
        }

        #[derive(Debug)]
        struct Lookup {
            ty: u16,
            _flag: u16,
            sub_idxs: Vec<usize>, // absolute offsets
        }
        fn read_lookup_list(reader: &mut ReaderBe) -> Vec<Lookup> {
            let idx = reader.idx();
            let count = reader.read16();
            (0..count)
                .map(|_| {
                    let off = reader.read16();
                    let lookup_idx = idx + off as usize;
                    let ret = reader.idx();
                    reader.goto(lookup_idx);
                    let ty = reader.read16();
                    let _flag = reader.read16();
                    let sub_count = reader.read16();
                    let sub_idxs = (0..sub_count)
                        .map(|_| lookup_idx + reader.read16() as usize)
                        .collect();
                    reader.goto(ret);
                    Lookup {
                        ty,
                        _flag,
                        sub_idxs,
                    }
                })
                .collect()
        }

        fn read_coverage(reader: &mut ReaderBe) -> Vec<u16> {
            let format = reader.read16();
            match format {
                1 => {
                    let count = reader.read16();
                    reader.read_arr16(count as usize)
                }
                2 => {
                    let range_count = reader.read16();
                    let mut glyphs = Vec::new();
                    for _ in 0..range_count {
                        let start = reader.read16();
                        let end = reader.read16();
                        let _start_idx = reader.read16();
                        for g in start..=end {
                            glyphs.push(g)
                        }
                    }
                    glyphs
                }
                _ => panic!("invalid coverage format: {format}"),
            }
        }

        fn read_val_record(reader: &mut ReaderBe, fmt: u16) -> i16 {
            assert!(fmt <= 0xF, "unsupported value record format: {fmt}");
            _ = if fmt & 0x01 != 0 { reader.read16() } else { 0 } as i16;
            _ = if fmt & 0x02 != 0 { reader.read16() } else { 0 } as i16;
            let x_adv = if fmt & 0x04 != 0 { reader.read16() } else { 0 } as i16;
            _ = if fmt & 0x08 != 0 { reader.read16() } else { 0 } as i16;
            x_adv
        }

        fn read_class_def(reader: &mut ReaderBe) -> HashMap<u16, u16> {
            let mut class_map = HashMap::new();
            let format = reader.read16();
            match format {
                1 => {
                    let start_glyph_idx = reader.read16();
                    let glyph_count = reader.read16();
                    for i in 0..glyph_count {
                        let class = reader.read16();
                        class_map.insert(start_glyph_idx + i, class);
                    }
                }
                2 => {
                    let class_range_count = reader.read16();
                    for _ in 0..class_range_count {
                        let start_glyph_idx = reader.read16();
                        let end_glyph_idx = reader.read16();
                        let class = reader.read16();
                        for glyph_idx in start_glyph_idx..=end_glyph_idx {
                            class_map.insert(glyph_idx, class);
                        }
                    }
                }
                _ => panic!("invalid class def format: {format}"),
            }
            class_map
        }

        fn read_pair_adjustements(reader: &mut ReaderBe) -> Vec<(u32, i16)> {
            let idx = reader.idx();
            let mut kernings = Vec::new();

            let format = reader.read16();

            let coverage_off = reader.read16();
            let ret = reader.idx();
            let coverage_idx = idx + coverage_off as usize;
            reader.goto(coverage_idx);
            let first_glyphs = read_coverage(reader);
            reader.goto(ret);

            let val_format1 = reader.read16();
            let val_format2 = reader.read16();

            match format {
                1 => {
                    let pair_set_count = reader.read16();
                    assert_eq!(
                        first_glyphs.len(),
                        pair_set_count as usize,
                        "coverage glyph count does not match pair set count"
                    );
                    let pair_set_offs = reader.read_arr16(pair_set_count as usize);
                    for (i, &ps_off) in pair_set_offs.iter().enumerate() {
                        let pair_set_idx = idx + ps_off as usize;
                        reader.goto(pair_set_idx);
                        let pair_val_count = reader.read16();
                        for _ in 0..pair_val_count {
                            let second_glyph = reader.read16();
                            let x_adv1 = read_val_record(reader, val_format1);
                            let x_adv2 = read_val_record(reader, val_format2);
                            let kern = x_adv1 + x_adv2;
                            let first_glyph = first_glyphs[i];
                            if kern != 0 {
                                kernings.push((
                                    ((first_glyph as u32) << 16) | second_glyph as u32,
                                    kern,
                                ))
                            }
                        }
                    }
                }
                2 => {
                    let class_def1_off = reader.read16();
                    let ret = reader.idx();
                    reader.goto(idx + class_def1_off as usize);
                    let class_def1 = read_class_def(reader);
                    reader.goto(ret);

                    let class_def2_off = reader.read16();
                    let ret = reader.idx();
                    reader.goto(idx + class_def2_off as usize);
                    let class_def2 = read_class_def(reader);
                    reader.goto(ret);

                    let class1_count = reader.read16();
                    let class2_count = reader.read16();

                    let record_count = class1_count as usize * class2_count as usize;
                    let mut value_records = Vec::with_capacity(record_count);
                    for _ in 0..record_count {
                        let v1 = read_val_record(reader, val_format1);
                        let v2 = read_val_record(reader, val_format2);
                        value_records.push((v1, v2));
                    }

                    for &first_glyph in &first_glyphs {
                        let class1 = class_def1.get(&first_glyph).copied().unwrap_or(0);
                        if class1 >= class1_count {
                            continue;
                        }

                        for second_glyph in 0..class_def2.len() as u16 {
                            let class2 = class_def2.get(&second_glyph).copied().unwrap_or(0);
                            if class2 >= class2_count {
                                continue;
                            }

                            let index = class1 as usize * class2_count as usize + class2 as usize;
                            if let Some((v1, v2)) = value_records.get(index) {
                                let kern = v1 + v2;
                                if kern != 0 {
                                    let key = ((first_glyph as u32) << 16) | second_glyph as u32;
                                    kernings.push((key, kern));
                                }
                            }
                        }
                    }
                }
                _ => panic!("invalid pair adjustement format: {format}"),
            }

            kernings
        }

        fn read_ext_positioning(reader: &mut ReaderBe) -> Vec<(u32, i16)> {
            let idx = reader.idx();
            let format = reader.read16();
            if format != 1 {
                panic!("unsupported extension positioning format");
            }
            let ext_lookup_ty = reader.read16();
            let ext_off = reader.read32();
            let subtable_idx = idx + ext_off as usize;
            reader.goto(subtable_idx);
            match ext_lookup_ty {
                2 => read_pair_adjustements(reader),
                _ => Default::default(),
            }
        }

        let mut kernings = HashMap::new();
        if table_offs.gpos == 0 {
            return kernings;
        }

        let gpos_idx = table_offs.gpos as usize;
        reader.goto(gpos_idx);

        let version = reader.read32();
        let _script_off = reader.read16();
        let feature_off = reader.read16();
        let lookup_off = reader.read16();
        if version == 0x00010001 {
            let _feature_var_off = reader.read32();
        }

        let feature_list_idx = gpos_idx + feature_off as usize;
        reader.goto(feature_list_idx);
        let features = read_feature_list(reader);
        let Some(kern_lookup_idxs) = features.get(&u32::from_be_bytes(*b"kern")) else {
            return kernings;
        };

        reader.goto(gpos_idx + lookup_off as usize);
        let lookups = read_lookup_list(reader);
        for &idx in kern_lookup_idxs {
            let idx = idx as usize;
            let lookup = &lookups[idx];
            match lookup.ty {
                2 => {
                    for &sub_idx in &lookup.sub_idxs {
                        reader.goto(sub_idx);
                        kernings.extend(read_pair_adjustements(reader));
                    }
                }
                9 => {
                    for &sub_idx in &lookup.sub_idxs {
                        reader.goto(sub_idx);
                        kernings.extend(read_ext_positioning(reader));
                    }
                }
                _ => {}
            }
        }
        kernings
    }

    fn read_hmetrics(
        reader: &mut ReaderBe,
        glyph_offs: &[u32],
        table_offs: &TableOffs,
    ) -> Hmetrics {
        reader.goto(table_offs.hhea as usize);
        reader.skip(4); // major/minor version
        let ascent = reader.read16() as i16;
        let descent = reader.read16() as i16;
        let line_gap = reader.read16() as i16;
        // max advance width, min left/right side bearing, xmax extent, caret slope rise/run
        // caret off, reserved64, metric data format (note: all 16 bits)
        reader.skip(27);
        let num_hmetrics = reader.read16() as usize;

        let num_glyphs = glyph_offs.len() - 1;
        reader.goto(table_offs.hmtx as usize);
        let mut glyph_hmetrics = (0..num_glyphs)
            .map(|_| {
                let advance_width = reader.read16() as i16;
                let left_side_bearing = reader.read16() as i16;
                (advance_width, left_side_bearing)
            })
            .collect::<Vec<_>>();
        // some fonts have mono-spaced glyphs at the end
        // which have same advance width as last
        for i in num_hmetrics..num_glyphs {
            glyph_hmetrics[i] = glyph_hmetrics[num_hmetrics - 1];
        }
        Hmetrics {
            ascent,
            descent,
            line_gap,
            glyph_hmetrics,
        }
    }

    fn read_vmetrics(
        reader: &mut ReaderBe,
        glyph_offs: &[u32],
        table_offs: &TableOffs,
    ) -> Vmetrics {
        reader.goto(table_offs.vhea as usize);
        reader.skip(4); // version
        let ascent = reader.read16() as i16;
        let descent = reader.read16() as i16;
        reader.skip(2); // line gap (reserved, set to 0)
        // advance height max, min top/bottom side bearing, y max extent
        // caret slope rise/run, caret offset, reserved64, metrid data format
        reader.skip(24);
        let num_vmetrics = reader.read16() as usize;

        let num_glyphs = glyph_offs.len() - 1;
        reader.goto(table_offs.vmtx as usize);
        let mut glyph_vmetrics = (0..num_glyphs)
            .map(|_| {
                let advance_height = reader.read16() as i16;
                let top_side_bearing = reader.read16() as i16;
                (advance_height, top_side_bearing)
            })
            .collect::<Vec<_>>();
        for i in num_vmetrics..num_glyphs {
            glyph_vmetrics[i] = glyph_vmetrics[num_vmetrics - 1];
        }
        Vmetrics {
            ascent,
            descent,
            glyph_vmetrics,
        }
    }

    fn read_glyphs(
        reader: &mut ReaderBe,
        glyph_offs: &[u32],
        glyph_hmetrics: &[(i16, i16)],
        glyph_vmetrics: &[(i16, i16)],
        idx2uni: &[char],
    ) -> Vec<GlyphData> {
        let mut glyphs = vec![GlyphData::default(); glyph_offs.len() - 1];
        for i in 0..glyphs.len() {
            if !idx2uni[i].is_ascii() || idx2uni[i].is_ascii_graphic() {
                glyphs[i] = Self::read_glyph(reader, glyph_offs, i as u16);
            }
            glyphs[i].metric.advance_width = glyph_hmetrics[i].0;
            glyphs[i].metric.left_side_bearing = glyph_hmetrics[i].1;
            glyphs[i].metric.advance_height = glyph_vmetrics[i].0;
            glyphs[i].metric.top_side_bearing = glyph_vmetrics[i].1;
            glyphs[i].index = i as u16;
        }
        glyphs
    }

    fn read_glyph(reader: &mut ReaderBe, glyph_offs: &[u32], glyph_idx: u16) -> GlyphData {
        let glyph_off = glyph_offs[glyph_idx as usize];
        if glyph_off == glyph_offs[glyph_idx as usize + 1] {
            return GlyphData {
                metric: GlyphMetrics::default(),
                points: Vec::new(),
                contour_end_idxs: Vec::new(),
                index: glyph_idx,
            };
        }
        reader.goto(glyph_off as usize);
        let contour_count = reader.read16() as i16;
        if contour_count == 0 {
            return GlyphData {
                metric: GlyphMetrics::default(),
                points: Vec::new(),
                contour_end_idxs: Vec::new(),
                index: glyph_idx,
            };
        }
        let is_simple = contour_count >= 0;
        if is_simple {
            Self::read_simple_glyph(reader, contour_count)
        } else {
            Self::read_compound_glyph(reader, glyph_offs)
        }
    }

    fn read_glyph_offs(
        reader: &mut ReaderBe,
        table_offs: &TableOffs,
        num_glyphs: u16,
        loc_bytes: u16,
    ) -> Vec<u32> {
        reader.goto(table_offs.loca as usize);
        (0..num_glyphs + 1)
            .map(|_| {
                table_offs.glyf
                    + if loc_bytes == 2 {
                        reader.read16() as u32 * 2
                    } else {
                        reader.read32()
                    }
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

        let contour_end_idxs = reader.read_arr16(contour_count as usize);
        let num_points = *contour_end_idxs.iter().max().unwrap_or(&0) + 1;

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
                left_side_bearing: 0,
                advance_height: 0,
                top_side_bearing: 0,
            },
            points,
            contour_end_idxs,
            index: 0,
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
                left_side_bearing: 0,
                advance_height: 0,
                top_side_bearing: 0,
            },
            points,
            contour_end_idxs,
            index: 0,
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
        if !args_xy {
            todo!("args1&2 are point idx to be matched, not offsets");
        }
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
            jhat_y = f2d14(reader.read16());
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

    fn read_idx2uni_mappings(reader: &mut ReaderBe, cmap_off: u32, num_glyphs: u16) -> Vec<char> {
        assert_ne!(num_glyphs, 0);
        let mut idx2uni = vec!['\0'; num_glyphs as usize];
        reader.goto(cmap_off as usize);
        let _version = reader.read16();
        let num_tables = reader.read16();
        let mut cmap_subtable_off = 0;
        let mut selected_uni_ver_id: i32 = -1;
        for _ in 0..num_tables {
            let platform_id = reader.read16();
            let encoding_id = reader.read16();
            let off = reader.read32();
            match platform_id {
                // unicode encoding
                0 => {
                    if matches!(encoding_id, 0 | 1 | 3 | 4)
                        && encoding_id as i32 > selected_uni_ver_id
                    {
                        cmap_subtable_off = off;
                        selected_uni_ver_id = encoding_id as i32;
                    }
                }
                1 | 2 => {}
                // microsoft encoding
                3 => {
                    if selected_uni_ver_id == -1 && matches!(encoding_id, 1 | 10) {
                        cmap_subtable_off = off;
                    }
                }
                _ => panic!("unsupported platform id: {platform_id}"),
            }
        }
        assert_ne!(
            cmap_subtable_off, 0,
            "font does not contain supported char map type"
        );
        reader.goto(cmap_off as usize + cmap_subtable_off as usize);
        let format = reader.read16();
        let mut has_read_missing_char_glyph = false;
        match format {
            4 => {
                let _len = reader.read16();
                let _lang_code = reader.read16();
                let seg_count = reader.read16() / 2;
                reader.skip(6); // search range, entry selector, range shift
                let end_codes = reader.read_arr16(seg_count as usize);
                reader.skip(2);
                let start_codes = reader.read_arr16(seg_count as usize);
                let id_deltas = reader.read_arr16(seg_count as usize);
                let id_range_offs = (0..seg_count)
                    .map(|_| (reader.read16(), reader.idx() - 2))
                    .collect::<Vec<_>>();
                for i in 0..start_codes.len() {
                    let end_code = end_codes[i];
                    let mut cur_code = start_codes[i];
                    if cur_code == u16::MAX {
                        break;
                    }
                    while cur_code <= end_code {
                        let mut glyph_idx;
                        if id_range_offs[i].0 == 0 {
                            glyph_idx = cur_code.wrapping_add(id_deltas[i]);
                        } else {
                            let range_off_loc = id_range_offs[i].1 + id_range_offs[i].0 as usize;
                            let glyph_idx_arr_loc =
                                2 * (cur_code - start_codes[i]) as usize + range_off_loc;
                            reader.goto(glyph_idx_arr_loc);
                            glyph_idx = reader.read16();
                            if glyph_idx != 0 {
                                glyph_idx = glyph_idx.wrapping_add(id_deltas[i]);
                            }
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
            }
            12 => {
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
            _ => panic!("unsupported font cmap format: {format}"),
        }
        if !has_read_missing_char_glyph {
            idx2uni[0] = '\u{65535}';
        }
        idx2uni
    }
}
