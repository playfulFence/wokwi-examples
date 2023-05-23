// use std::sync::mpsc::channel;
use std::{thread, time::*, ptr, string::String};
use std::str;
use std::result::Result::Ok;

use anyhow::*;
use log::*;

// Common IDF stuff
use esp_idf_hal::prelude::*;
use esp_idf_hal::modem::*;
use esp_idf_hal::peripheral::*;
use esp_idf_sys::time_t;
use esp_idf_sys::time;
use esp_idf_sys::link_patches;

// Time stuff
use esp_idf_svc::systime::EspSystemTime;

use time::OffsetDateTime;
use time::macros::offset;

use esp_idf_svc::sntp;
use esp_idf_svc::sntp::SyncStatus;

// Graphic part
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;
use embedded_graphics::image::Image;
use display_interface_spi::SPIInterfaceNoCS;

// Fonts and image
use profont::{PROFONT_24_POINT, PROFONT_18_POINT};
use tinybmp::Bmp;

// Wi-Fi
use embedded_svc::wifi::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::wifi::*;
use esp_idf_svc::wifi::EspWifi;

use esp_idf_svc::{
    log::EspLogger,
};

// RustZX spectrum stuff 
use rustzx_core::zx::video::colors::ZXBrightness;
use rustzx_core::zx::video::colors::ZXColor;

mod display;


const textStyle: TextStyle = TextStyleBuilder::new()
    .alignment(embedded_graphics::text::Alignment::Center)
    .baseline(embedded_graphics::text::Baseline::Middle)
    .build();

const WIFI_SSID: &str = "Wokwi-GUEST";
const WIFI_PASS: &str = "";

fn main() -> Result<()> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Set up peripherals and display
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let mut dp = display::create!(peripherals)?;

    show_logo(&mut dp);
    wifi_image(&mut dp, false, display::color_conv);

    wifi_connecting(&mut dp, false, display::color_conv);
    info!(
        "About to initialize WiFi (SSID: {}, PASS: {})",
        WIFI_SSID, WIFI_PASS
    );

    let mut wifi = wifi(peripherals.modem, sysloop.clone(), WIFI_SSID, WIFI_PASS)?;

    wifi_connecting(&mut dp, true, display::color_conv);

    /* Unsafe section is used since it's required, if you're using C functions and datatypes */
    unsafe {
        let sntp = sntp::EspSntp::new_default()?;
        info!("SNTP initialized, waiting for status!");

        while sntp.get_sync_status() != SyncStatus::Completed {}

        info!("SNTP status received!");

        let timer: *mut time_t = ptr::null_mut();

        let mut timestamp = esp_idf_sys::time(timer);

        let mut actual_date = OffsetDateTime::from_unix_timestamp(timestamp as i64)?
            .to_offset(offset!(+2))
            .date();

        info!(
            "{} - {} - {}",
            actual_date.to_calendar_date().2,
            actual_date.to_calendar_date().1,
            actual_date.to_calendar_date().0
        );

        let mut date_str = format!(
            "{}-{}-{}",
            actual_date.to_calendar_date().2,
            actual_date.to_calendar_date().1,
            actual_date.to_calendar_date().0
        );

        let mut now: u64 = 0;
        let mut time_buf: u64 = 0;

        dateFlush(&mut dp, &date_str, display::color_conv);

        weekdayFlush(
            &mut dp,
            &actual_date.weekday().to_string(),
            display::color_conv,
        );

        let i2c = peripherals.i2c0;

        loop {
            timestamp = esp_idf_sys::time(timer);

            let mut rawTime =
                OffsetDateTime::from_unix_timestamp(timestamp as i64)?.to_offset(offset!(+2));

            timeFlush(
                &mut dp,
                &rawTime.time().to_string()[0..(rawTime.time().to_string().len() - 2)].to_string(),
                display::color_conv,
            );

            if actual_date != rawTime.date() {
                actual_date = rawTime.date();
                date_str = format!(
                    "{}-{}-{}",
                    actual_date.to_calendar_date().2,
                    actual_date.to_calendar_date().1,
                    actual_date.to_calendar_date().0
                );

                dateFlush(&mut dp, &date_str, display::color_conv);

                weekdayFlush(
                    &mut dp,
                    &actual_date.weekday().to_string(),
                    display::color_conv,
                );
            }

            thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}

fn timeFlush<D>(
    display: &mut D,
    toPrint: &String,
    color_conv: fn(ZXColor, ZXBrightness) -> D::Color,
) -> anyhow::Result<()>
where
    D: DrawTarget + Dimensions,
{
    Rectangle::with_center(
        display.bounding_box().center() + Size::new(0, 15),
        Size::new(132, 40),
    )
    .into_styled(
        PrimitiveStyleBuilder::new()
            .fill_color(color_conv(ZXColor::White, ZXBrightness::Normal))
            .stroke_color(color_conv(ZXColor::White, ZXBrightness::Normal))
            .stroke_width(1)
            .build(),
    )
    .draw(display);

    Text::with_text_style(
        &toPrint,
        display.bounding_box().center() + Size::new(0, 10), //(display.bounding_box().size.height - 10) as i32 / 2),
        MonoTextStyle::new(
            &PROFONT_24_POINT,
            color_conv(ZXColor::Black, ZXBrightness::Normal),
        ),
        textStyle,
    )
    .draw(display);

    Ok(())
}

fn dateFlush<D>(
    display: &mut D,
    toPrint: &String,
    color_conv: fn(ZXColor, ZXBrightness) -> D::Color,
) -> anyhow::Result<()>
where
    D: DrawTarget + Dimensions,
{
    Rectangle::new(Point::zero(), Size::new(170, 30))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(color_conv(ZXColor::White, ZXBrightness::Normal))       /* for date in top-left of screen*/
                .stroke_color(color_conv(ZXColor::White, ZXBrightness::Normal))
                .stroke_width(1)
                .build(),
        )
    .draw(display);

    Text::with_alignment(
        &toPrint,
        Point::new(5, 20), //(display.bounding_box().size.height - 10) as i32 / 2),
        MonoTextStyle::new(
            &PROFONT_18_POINT,
            color_conv(ZXColor::Black, ZXBrightness::Normal),
        ),
        Alignment::Left,
    )
    .draw(display);

    Ok(())
}

fn weekdayFlush<D>(
    display: &mut D,
    toPrint: &String,
    color_conv: fn(ZXColor, ZXBrightness) -> D::Color,
) -> anyhow::Result<()>
where
    D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565> + Dimensions,
{
    Rectangle::with_center(
        display.bounding_box().center() - Size::new(0, 20),
        Size::new(140, 30),
    )
    .into_styled(
        PrimitiveStyleBuilder::new()
            .fill_color(color_conv(ZXColor::White, ZXBrightness::Normal))
            .stroke_color(color_conv(ZXColor::White, ZXBrightness::Normal))
            .stroke_width(1)
            .build(),
    )
    .draw(display);

    Text::with_text_style(
        &toPrint,
        display.bounding_box().center() - Size::new(0, 25), //(display.bounding_box().size.height - 10) as i32 / 2),
        MonoTextStyle::new(
            &PROFONT_24_POINT,
            color_conv(ZXColor::Black, ZXBrightness::Normal),
        ),
        textStyle,
    )
    .draw(display);

    Ok(())
}

fn show_logo<D>(display: &mut D) -> anyhow::Result<()>
where
    D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>  + Dimensions,
{
    info!("Welcome!");

    /* big logo at first */
    display.clear(display::color_conv(ZXColor::White, ZXBrightness::Normal).into());
    let bmp = Bmp::<Rgb565>::from_slice(include_bytes!("/home/esp/assets/esp-rs-big.bmp")).unwrap();
    Image::new(&bmp, display.bounding_box().center() - Size::new(100, 100)).draw(display);

    thread::sleep(Duration::from_secs(5));

    /* than small */
    display.clear(display::color_conv(ZXColor::White, ZXBrightness::Normal).into());
    let bmp = Bmp::<Rgb565>::from_slice(include_bytes!("/home/esp/assets/esp-rs-small.bmp")).unwrap();
    Image::new(
        &bmp,
        Point::new(0, display.bounding_box().size.height as i32 - 50),
    )
    .draw(display);

    Ok(())
}

fn wifi(
    modem: impl esp_idf_hal::peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    wifi_ssid : &str, 
    wifi_password : &str, 
) -> Result<Box<EspWifi<'static>>> {
    use std::net::Ipv4Addr;

    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), None)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == wifi_ssid);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            wifi_ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            wifi_ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: wifi_ssid.into(),
            password: wifi_password.into(),
            auth_method: AuthMethod::None,
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    wifi.start()?;

    info!("Starting wifi...");

    if !WifiWait::new(&sysloop)?
        .wait_with_timeout(Duration::from_secs(20), || wifi.is_started().unwrap())
    {
        bail!("Wifi did not start");
    }

    info!("Connecting wifi...");

    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
        Duration::from_secs(20),
        || {
            wifi.is_connected().unwrap()
                && wifi.sta_netif().get_ip_info().unwrap().ip != Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        bail!("Wifi did not connect or did not receive a DHCP lease");
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(wifi)
}
/* if this bool is true => wifi connected */
fn wifi_connecting<D>(
    display: &mut D,
    connected: bool,
    color_conv: fn(ZXColor, ZXBrightness) -> D::Color,
) -> anyhow::Result<()>
where
    D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>  + Dimensions,
{
    Rectangle::with_center(
        display.bounding_box().center(),
        Size::new(display.bounding_box().size.width, 80),
    )
    .into_styled(
        PrimitiveStyleBuilder::new()
            .fill_color(color_conv(ZXColor::White, ZXBrightness::Normal))
            .stroke_color(color_conv(ZXColor::White, ZXBrightness::Normal))
            .stroke_width(1)
            .build(),
    )
    .draw(display);

    if connected {
        Text::with_text_style(
            "Wi-Fi connected",
            display.bounding_box().center() - Size::new(0, 25), //(display.bounding_box().size.height - 10) as i32 / 2),
            MonoTextStyle::new(
                &PROFONT_24_POINT,
                color_conv(ZXColor::Black, ZXBrightness::Normal),
            ),
            textStyle,
        )
        .draw(display);

        wifi_image(display, true, color_conv);

        thread::sleep(Duration::from_secs(2));

        Rectangle::with_center(
            display.bounding_box().center(),
            Size::new(display.bounding_box().size.width, 80),
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(color_conv(ZXColor::White, ZXBrightness::Normal))
                .stroke_color(color_conv(ZXColor::White, ZXBrightness::Normal))
                .stroke_width(1)
                .build(),
        )
        .draw(display);
    } else {
        Text::with_text_style(
            "Connecting Wi-Fi...",
            display.bounding_box().center() - Size::new(0, 25), //(display.bounding_box().size.height - 10) as i32 / 2),
            MonoTextStyle::new(
                &PROFONT_24_POINT,
                color_conv(ZXColor::Black, ZXBrightness::Normal),
            ),
            textStyle,
        )
        .draw(display);
    }

    Ok(())
}

/* if this bool is true => draw "WiFi connected image" */
/* otherwise - overcrossed WiFi image */
fn wifi_image<D>(
    display: &mut D,
    wifi: bool,
    color_conv: fn(ZXColor, ZXBrightness) -> D::Color,
) -> anyhow::Result<()>
where
    D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>  + Dimensions,
{
    if wifi {
        Rectangle::new(
            Point::new(50, display.bounding_box().size.height as i32 - 50),
            Size::new(50, 50),
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(color_conv(ZXColor::White, ZXBrightness::Normal))
                .stroke_color(color_conv(ZXColor::White, ZXBrightness::Normal))
                .stroke_width(1)
                .build(),
        )
        .draw(display);
        let bmp = Bmp::<Rgb565>::from_slice(include_bytes!("/home/esp/assets/wifi.bmp")).unwrap();
        Image::new(
            &bmp,
            Point::new(53, display.bounding_box().size.height as i32 - 50),
        )
        .draw(display);
    } else {
        let bmp =
            Bmp::<Rgb565>::from_slice(include_bytes!("/home/esp/assets/wifi_not_connected.bmp")).unwrap();
        Image::new(
            &bmp,
            Point::new(53, display.bounding_box().size.height as i32 - 50),
        )
        .draw(display);
    }

    Ok(())
}
