use core::str::FromStr;

use embedded_graphics::{
    geometry::AnchorX,
    image::Image,
    mono_font::{
        ascii::{FONT_6X12, FONT_6X13_BOLD},
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

use crate::state::HomeItem;

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
