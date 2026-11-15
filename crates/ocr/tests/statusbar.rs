use ocr::{Fonts, GlyphSet, learn, locate, read_frame};

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
