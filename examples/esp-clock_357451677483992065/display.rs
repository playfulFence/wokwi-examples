#![allow(unused_imports)]

use anyhow::*;
use log::*;

use esp_idf_hal::gpio;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;
use esp_idf_hal::{delay, i2c};

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;

use ili9341;

use rustzx_core::zx::video::colors::ZXBrightness;
use rustzx_core::zx::video::colors::ZXColor;

// use ssd1306::mode::DisplayConfig;

macro_rules! create {
    ($peripherals:expr) => {{
        let result = display::esp32c3_create_display_ili9341(
            $peripherals.pins.gpio4,
            $peripherals.pins.gpio3,
            $peripherals.pins.gpio18,
            $peripherals.spi2,
            $peripherals.pins.gpio6,
            $peripherals.pins.gpio7,
            $peripherals.pins.gpio2,
        );
        result
    }};
}

pub(crate) use create;

pub(crate) fn esp32c3_create_display_ili9341<'d>(
    backlight: gpio::Gpio4,
    dc: gpio::Gpio3,
    rst: gpio::Gpio18,
    spi: spi::SPI2,
    sclk: gpio::Gpio6,
    sdo: gpio::Gpio7,
    cs: gpio::Gpio2,
) -> Result<
     ili9341::Ili9341<
        SPIInterfaceNoCS<
            spi::SpiDeviceDriver<'d, 
                spi::SpiDriver<'d>
            >, 
            gpio::PinDriver<'d,
                gpio::Gpio3, 
                gpio::Output
            >
        >,
        gpio::PinDriver<'d,
                gpio::Gpio18, 
                gpio::Output
        >,  
    >,
> {

  use esp_idf_hal::{spi::SpiDeviceDriver, gpio::OutputPin};

    // Kaluga needs customized screen orientation commands
    // (not a surprise; quite a few ILI9341 boards need these as evidences in the TFT_eSPI & lvgl ESP32 C drivers)
    // Display orientation: https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf
    // Page 209
    pub enum KalugaOrientation {
        Portrait,
        PortraitFlipped,
        Landscape,
        LandscapeVericallyFlipped,
        LandscapeFlipped,
    }

    impl ili9341::Mode for KalugaOrientation {
        fn mode(&self) -> u8 {
            match self {
                Self::Portrait => 0,
                Self::LandscapeVericallyFlipped => 0x20,
                Self::Landscape => 0x20 | 0x40,
                Self::PortraitFlipped => 0x80 | 0x40,
                /* this is used for Wokwi simulation, "| 0x08" is used to invert colors to correct */
                Self::LandscapeFlipped => 0x80 | 0x20 | 0x08,
            }
        }

        fn is_landscape(&self) -> bool {
            matches!(
                self,
                Self::Landscape | Self::LandscapeFlipped | Self::LandscapeVericallyFlipped
            )
        }
    }

    info!("About to initialize the ESP32C3 ILI9341 SPI LED driver");

    let config = <spi::config::Config as Default>::default().baudrate(40.MHz().into());
    //.bit_order(spi::config::BitOrder::MSBFirst);

    let mut backlight = gpio::PinDriver::output(backlight)?;
    backlight.set_low()?;
    

    let di = SPIInterfaceNoCS::new(
        spi::SpiDeviceDriver::new_single(
            spi,
            sclk,
            sdo,
            Option::<gpio::AnyIOPin>::None,
            spi::Dma::Disabled,
            Some(cs),
            &spi::SpiConfig::new().baudrate(40.MHz().into()),
        )?,
        gpio::PinDriver::output(dc)?,
    );

    let reset = gpio::PinDriver::output(rst)?;

    ili9341::Ili9341::new(
        di,
        reset,
        &mut delay::Ets,
        KalugaOrientation::LandscapeFlipped, // uncomment this line and comment the line above for correct Wokwi simulation
        ili9341::DisplaySize240x320,
    ).map_err(|e| anyhow!("Failed to init display"))
}

pub(crate) fn color_conv(color: ZXColor, _brightness: ZXBrightness) -> Rgb565 {
    match color {
        ZXColor::Black => Rgb565::BLACK,
        ZXColor::Blue => Rgb565::BLUE,
        ZXColor::Red => Rgb565::RED,
        ZXColor::Purple => Rgb565::MAGENTA,
        ZXColor::Green => Rgb565::GREEN,
        ZXColor::Cyan => Rgb565::CYAN,
        ZXColor::Yellow => Rgb565::YELLOW,
        ZXColor::White => Rgb565::WHITE,
    }
}
