use core::{fmt::Write, str::FromStr};

use embedded_graphics::{
    geometry::AnchorX,
    image::Image,
    mono_font::{
        ascii::{FONT_5X7, FONT_6X12, FONT_6X13_BOLD},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{
        CornerRadii, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
        StrokeAlignment, StyledDrawable,
    },
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};
use embedded_sdmmc::ShortFileName;
use heapless::String;
use ssd1306::{
    mode::BufferedGraphicsMode, prelude::I2CInterface, size::DisplaySize128x64, Ssd1306,
};
use stm32f1xx_hal::{
    gpio::{Alternate, OpenDrain, Pin},
    i2c::BlockingI2c,
    pac::I2C1,
};
use tinybmp::Bmp;

use crate::{
    can::{Bitrate, EmissionMode},
    state::HomeItem,
};

pub type Display = Ssd1306<
    I2CInterface<
        BlockingI2c<
            I2C1,
            (
                Pin<'B', 6, Alternate<OpenDrain>>,
                Pin<'B', 7, Alternate<OpenDrain>>,
            ),
        >,
    >,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

const DISPLAY_WIDTH: u32 = 128;
const DISPLAY_HEIGHT: u32 = 64;

const TEXT_HEIGHT: i32 = 12;
const TEXT_LINE_1: i32 = 1 * 12 - 1;
const TEXT_LINE_2: i32 = 2 * 12 + 0;
const TEXT_LINE_3: i32 = 3 * 12 + 1;
const TEXT_LINE_4: i32 = 4 * 12 + 2;
const TEXT_LINE_5: i32 = 5 * 12 + 3;

static DEFAULT_TEXT_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X12)
    .text_color(BinaryColor::On)
    .background_color(BinaryColor::Off)
    .build();
static HIGHLIGHTED_TEXT_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X12)
    .text_color(BinaryColor::On)
    .background_color(BinaryColor::Off)
    .underline()
    .build();
static TITLE_TEXT_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X13_BOLD)
    .text_color(BinaryColor::Off)
    .background_color(BinaryColor::On)
    .build();
static HEADER_TEXT_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X12)
    .text_color(BinaryColor::Off)
    .background_color(BinaryColor::On)
    .build();
static SMALL_TEXT_STYLE: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_5X7)
    .text_color(BinaryColor::On)
    .background_color(BinaryColor::Off)
    .build();

static CENTER_MIDDLE: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Middle)
    .build();
static CENTER_BOTTOM: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Bottom)
    .build();
static LEFT_BOTTOM: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Left)
    .baseline(Baseline::Bottom)
    .build();
static RIGHT_BOTTOM: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Right)
    .baseline(Baseline::Bottom)
    .build();
static RIGHT_TOP: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Right)
    .baseline(Baseline::Top)
    .build();

static BUTTON_STROKE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .stroke_alignment(StrokeAlignment::Inside)
    .build();
static BUTTON_STROKE_SELECTED: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(2)
    .stroke_alignment(StrokeAlignment::Inside)
    .build();

fn draw_whole_line(text: Text<MonoTextStyle<BinaryColor>>, display: &mut Display) {
    let _ = text
        .bounding_box()
        .resized_width(DISPLAY_WIDTH + 2, AnchorX::Center)
        .into_styled(PrimitiveStyle::with_fill(
            text.character_style.background_color.unwrap_or_default(),
        ))
        .draw(display);
    let _ = text.draw(display);
}

fn draw_left_hint(display: &mut Display, hint: &str) {
    let text_hint = Text::with_text_style(
        hint,
        Point::new(7, (DISPLAY_HEIGHT - 1) as i32),
        DEFAULT_TEXT_STYLE,
        LEFT_BOTTOM,
    );
    let hint_outline = RoundedRectangle::new(
        Rectangle::new(
            text_hint.bounding_box().top_left - Point::new(9, 1),
            text_hint.bounding_box().size + Size::new(9 + 2, 1 + 2),
        ),
        CornerRadii::new(Size::new_equal(4)),
    );

    let _ = Image::new(
        &Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/left.bmp")).unwrap(),
        Point::new(0, DISPLAY_HEIGHT as i32 - 12),
    )
    .draw(display);
    let _ = text_hint.draw(display);
    let _ = hint_outline.draw_styled(&BUTTON_STROKE, display);
}
fn draw_right_hint(display: &mut Display, hint: &str) {
    let text_hint = Text::with_text_style(
        hint,
        Point::new((DISPLAY_WIDTH - 8) as i32, (DISPLAY_HEIGHT - 1) as i32),
        DEFAULT_TEXT_STYLE,
        RIGHT_BOTTOM,
    );
    let hint_outline = RoundedRectangle::new(
        Rectangle::new(
            text_hint.bounding_box().top_left - Point::new(3, 1),
            text_hint.bounding_box().size + Size::new(3 + 9, 1 + 2),
        ),
        CornerRadii::new(Size::new_equal(4)),
    );

    let _ = Image::new(
        &Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/right.bmp")).unwrap(),
        Point::new(DISPLAY_WIDTH as i32 - 7, DISPLAY_HEIGHT as i32 - 12),
    )
    .draw(display);
    let _ = text_hint.draw(display);
    let _ = hint_outline.draw_styled(&BUTTON_STROKE, display);
}
fn draw_center_hint(display: &mut Display, hint: &str, x_displacement: i32) {
    let text_hint = Text::with_text_style(
        hint,
        Point::new(
            (DISPLAY_WIDTH / 2 + 3) as i32 + x_displacement,
            (DISPLAY_HEIGHT - 1) as i32,
        ),
        DEFAULT_TEXT_STYLE,
        CENTER_BOTTOM,
    );
    let hint_outline = RoundedRectangle::new(
        Rectangle::new(
            text_hint.bounding_box().top_left - Point::new(7 + 2, 1),
            text_hint.bounding_box().size + Size::new((7 + 2) + 2, 1 + 2),
        ),
        CornerRadii::new(Size::new_equal(4)),
    );

    let _ = Image::new(
        &Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/ok.bmp")).unwrap(),
        Point::new(
            text_hint.bounding_box().top_left.x - 7,
            DISPLAY_HEIGHT as i32 - 12,
        ),
    )
    .draw(display);
    let _ = text_hint.draw(display);
    let _ = hint_outline.draw_styled(&BUTTON_STROKE, display);
}

pub fn draw_home(display: &mut Display, selected_item: &HomeItem) {
    let mut title = String::<16>::from_str("CANary v").unwrap();
    title.push_str(env!("CARGO_PKG_VERSION")).unwrap();

    draw_whole_line(
        Text::with_text_style(
            &title,
            Point::new((DISPLAY_WIDTH / 2) as i32, TEXT_LINE_1),
            TITLE_TEXT_STYLE,
            CENTER_BOTTOM,
        ),
        display,
    );

    let emit_button_pos = Point::new((DISPLAY_WIDTH / 4) as i32, 37);
    let capt_button_pos = Point::new((DISPLAY_WIDTH / 4 * 3) as i32, 37);
    let home_button_size = Size::new(48, 48);
    let (emit_styles, capt_styles) = match selected_item {
        HomeItem::Emit => (
            (&BUTTON_STROKE_SELECTED, HIGHLIGHTED_TEXT_STYLE),
            (&BUTTON_STROKE, DEFAULT_TEXT_STYLE),
        ),
        HomeItem::Capture => (
            (&BUTTON_STROKE, DEFAULT_TEXT_STYLE),
            (&BUTTON_STROKE_SELECTED, HIGHLIGHTED_TEXT_STYLE),
        ),
    };

    let _ = RoundedRectangle::new(
        Rectangle::with_center(emit_button_pos, home_button_size),
        CornerRadii::new(Size::new_equal(8)),
    )
    .draw_styled(emit_styles.0, display);
    let _ =
        Text::with_text_style("Emit", emit_button_pos, emit_styles.1, CENTER_MIDDLE).draw(display);

    let _ = RoundedRectangle::new(
        Rectangle::with_center(capt_button_pos, home_button_size),
        CornerRadii::new(Size::new_equal(8)),
    )
    .draw_styled(capt_styles.0, display);
    let _ = Text::with_text_style("Capture", capt_button_pos, capt_styles.1, CENTER_MIDDLE)
        .draw(display);
}

pub fn draw_file_selection(
    display: &mut Display,
    current_dir: Option<&ShortFileName>,
    content: &[(bool, ShortFileName)],
    selected_index: usize,
) {
    let file_icon = Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/file.bmp")).unwrap();
    let dir_icon = Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/directory.bmp")).unwrap();
    let selected_icon =
        Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/chevron_right.bmp")).unwrap();
    let scroll_icon =
        Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/chevrons_vertical.bmp")).unwrap();

    let mut dir_str = String::<16>::new();
    if let Some(current_dir) = current_dir {
        dir_str
            .write_fmt(format_args!("{}", current_dir))
            .expect("ShortFileNames.len() <= 12");
        dir_str.make_ascii_lowercase();
    } else {
        dir_str.push_str("root").unwrap();
    }
    draw_whole_line(
        Text::with_text_style(
            &dir_str,
            Point::new((DISPLAY_WIDTH / 2) as i32, TEXT_LINE_1),
            HEADER_TEXT_STYLE,
            CENTER_BOTTOM,
        ),
        display,
    );

    draw_left_hint(display, "back");
    draw_center_hint(display, "select", -4);
    draw_right_hint(display, "enter");

    if content.len() == 0 {
        return;
    }
    if content.len() > 3 {
        let _ = Image::new(
            &scroll_icon,
            Point::new(DISPLAY_WIDTH as i32 - 12, TEXT_LINE_3 - 11),
        )
        .draw(display);
    }

    let (content, highlighted_i) = match selected_index {
        0 => (&content[0..=2.min(content.len() - 1)], 0),
        n if content.len() <= 3 => (content, n),
        n if n == content.len() - 1 => (&content[content.len() - 3..content.len()], 2),
        n => (&content[n - 1..=n + 1], 1),
    };

    for (i, (is_dir, name)) in content.iter().enumerate() {
        let _ = Image::new(
            if i == highlighted_i as usize {
                &selected_icon
            } else if *is_dir {
                &dir_icon
            } else {
                &file_icon
            },
            Point::new(0, TEXT_LINE_2 + 13 * i as i32 - 11),
        )
        .draw(display);

        let mut name_str = String::<16>::new();
        name_str
            .write_fmt(format_args!("{}", name))
            .expect("ShortFileNames.len() <= 12");
        name_str.make_ascii_lowercase();
        let _ = Text::with_text_style(
            &name_str,
            Point::new(13, TEXT_LINE_2 + 13 * i as i32),
            if i == highlighted_i as usize {
                HIGHLIGHTED_TEXT_STYLE
            } else {
                DEFAULT_TEXT_STYLE
            },
            LEFT_BOTTOM,
        )
        .draw(display);
    }
}

pub fn draw_emission(
    display: &mut Display,
    selected: &ShortFileName,
    running: bool,
    count: u8,
    bitrate: &Bitrate,
    mode: &EmissionMode,
    success_count: u32,
) {
    let emit_icon = Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/emit.bmp")).unwrap();
    let scroll_icon =
        Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/chevrons_vertical.bmp")).unwrap();
    let play_icon = Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/play.bmp")).unwrap();
    let pause_icon = Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/pause.bmp")).unwrap();
    let stop_icon = Bmp::<BinaryColor>::from_slice(include_bytes!("./icons/stop.bmp")).unwrap();

    let mut file_str = String::<16>::new();
    file_str
        .write_fmt(format_args!("{}", selected))
        .expect("ShortFileName.len() <= 12");
    file_str.make_ascii_lowercase();
    draw_whole_line(
        Text::with_text_style(
            &file_str,
            Point::new((DISPLAY_WIDTH / 2) as i32, TEXT_LINE_1),
            HEADER_TEXT_STYLE,
            CENTER_BOTTOM,
        ),
        display,
    );
    let _ = Image::new(&emit_icon, Point::zero()).draw(display);

    if running {
        draw_center_hint(display, "Stop", -4);
    } else {
        draw_left_hint(display, "Exit");
        draw_center_hint(display, "Start", -7);
        draw_right_hint(display, "Params");
    }

    let mut count_str = String::<14>::new();
    if count == 0 {
        count_str.push_str("Repeating xINF").unwrap();
    } else {
        count_str
            .write_fmt(format_args!("Repeating x{:->3}", count))
            .unwrap();
    }
    let mut bitrate_str = String::<17>::new();
    bitrate_str
        .write_fmt(format_args!("Bitrate: {:4}kbps", *bitrate as u32 / 1000))
        .unwrap();
    let mut mode_str = String::<15>::new();
    mode_str
        .write_fmt(format_args!("Mode: {:?}", mode))
        .unwrap();

    let _ = Image::new(&scroll_icon, Point::new(5 * 14 - 2, TEXT_LINE_2 - 10)).draw(display);
    let _ = Text::with_text_style(
        &count_str,
        Point::new(0, TEXT_LINE_2),
        SMALL_TEXT_STYLE,
        LEFT_BOTTOM,
    )
    .draw(display);
    let _ = Text::with_text_style(
        &bitrate_str,
        Point::new(0, TEXT_LINE_3),
        SMALL_TEXT_STYLE,
        LEFT_BOTTOM,
    )
    .draw(display);
    let _ = Text::with_text_style(
        &mode_str,
        Point::new(0, TEXT_LINE_3 + 8),
        SMALL_TEXT_STYLE,
        LEFT_BOTTOM,
    )
    .draw(display);

    let _ = Image::new(
        if running {
            &pause_icon
        } else if success_count == 0 {
            &play_icon
        } else {
            &stop_icon
        },
        Point::new(DISPLAY_WIDTH as i32 - 16 - 12, 18),
    )
    .draw(display);

    let mut success_str = String::<16>::new();
    let _ = Text::with_text_style(
        if running {
            "Running"
        } else if success_count == 0 {
            "Standby"
        } else {
            success_str
                .write_fmt(format_args!("Sent {}\nframes", success_count % 10000))
                .unwrap();
            &success_str
        },
        Point::new(DISPLAY_WIDTH as i32 - 16 / 2 - 12, TEXT_LINE_3 + 4),
        SMALL_TEXT_STYLE,
        CENTER_BOTTOM,
    )
    .draw(display);
}
