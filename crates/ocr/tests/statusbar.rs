use image::imageops::{self, FilterType};
use ocr::{Fonts, GlyphSet, View, learn, locate, read_frame, scale};

fn fixture() -> image::RgbaImage {
    image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/statusbar-1920x1228.png"))
        .unwrap()
        .to_rgba8()
}

#[test]
fn reads_exp_and_level_from_real_capture() {
    let img = fixture();
    let fonts = Fonts::builtin();
    let layout = locate::locate(&img, &fonts.labels).expect("status bar located");
    let frame = read_frame(&img, &layout, &fonts);
    assert_eq!(frame.exp_line.text, "54,244[31.14%]");
    assert_eq!(frame.exp, Some((54_244, 31.14)));
    assert_eq!(frame.level, Some(35));
}

#[test]
fn infers_and_learns_a_missing_digit() {
    let img = fixture();
    let mut fonts = Fonts::builtin();
    let kept: Vec<_> = fonts.status.templates().iter().filter(|t| t.ch != '2').cloned().collect();
    let mut set = GlyphSet::default();
    for t in kept {
        set.add(t.ch, &ocr::Glyph { x: 0, top: t.top, mask: t.mask });
    }
    fonts.status = set;

    let layout = locate::locate(&img, &fonts.labels).unwrap();
    let frame = read_frame(&img, &layout, &fonts);
    assert_eq!(frame.exp_line.text, "54,?44[31.14%]");
    assert_eq!(frame.exp, None);

    let guess = learn::infer_exp_digits(&frame.exp_line, 174_194).unwrap();
    assert_eq!(learn::learn_expected(&frame.exp_line, &guess, &mut fonts.status), 1);
    assert_eq!(read_frame(&img, &layout, &fonts).exp, Some((54_244, 31.14)));
}

#[test]
fn reads_native_capture_through_detect() {
    let img = fixture();
    let fonts = Fonts::builtin();
    let (view, layout) = scale::detect(&img, &fonts).expect("status bar located");
    assert_eq!(view, View::NATIVE);
    assert_eq!(read_frame(&view.band(&img), &layout, &fonts).exp, Some((54_244, 31.14)));
}

#[test]
fn reads_capture_stretched_by_display_scaling() {
    let img = fixture();
    let fonts = Fonts::builtin();
    // Bilinear stretch like Windows DPI virtualization, with a sub-pixel offset from cropping.
    for (s, cut_left, cut_bottom) in [(1.25, 1, 2), (1.5, 0, 1), (2.0, 3, 0)] {
        let (w, h) = img.dimensions();
        let big = imageops::resize(&img, (w as f32 * s).round() as u32, (h as f32 * s).round() as u32, FilterType::Triangle);
        let shot = imageops::crop_imm(&big, cut_left, 0, big.width() - cut_left, big.height() - cut_bottom).to_image();
        let (view, layout) = scale::detect(&shot, &fonts).unwrap_or_else(|| panic!("not found at {s}"));
        assert_eq!(view.scale, s);
        let frame = read_frame(&view.band(&shot), &layout, &fonts);
        assert_eq!(frame.exp, Some((54_244, 31.14)), "scale {s}: {:?}", frame.exp_line.text);
        assert_eq!(frame.level, Some(35), "scale {s}");
        assert_eq!(scale::locate_in(&shot, &fonts.labels, view), Some(layout));
    }
}
