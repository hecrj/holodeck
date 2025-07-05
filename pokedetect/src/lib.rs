use bytes::Bytes;
use opencv::core::{AlgorithmHint, Mat, MatTraitConst, Rect, Scalar};
use opencv::imgproc::{self, INTER_LINEAR};
use opencv::videoio::{CAP_PROP_FOURCC, CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH};
use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::Stream;
use tokio_stream::wrappers;

use std::collections::HashMap;
use std::fmt;
use std::io;

#[derive(Debug, Clone)]
pub enum Event {
    Captured(Image),
    Detected { set: String, number: String },
}

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Bytes,
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba", &self.rgba.len())
            .finish()
    }
}

pub fn run() -> impl Stream<Item = Event> {
    let (sender, receiver) = mpsc::channel(1);

    let _handle = task::spawn_blocking(move || {
        log::info!("Starting capture");

        if let Err(error) = worker(sender) {
            log::error!("{error}");
        }

        log::info!("Capture finished");
    });

    wrappers::ReceiverStream::new(receiver)
}

pub fn worker(sender: mpsc::Sender<Event>) -> Result<(), anywho::Error> {
    use leptess::LepTess;
    use opencv::core::{
        AlgorithmHint, Mat, MatTraitConst, MatTraitConstManual, Point, Rect, Vector,
    };
    use opencv::imgproc;
    use opencv::videoio::{self, VideoCaptureTrait, VideoCaptureTraitConst};

    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 is usually the default cam

    let fourcc = videoio::VideoWriter::fourcc('M', 'J', 'P', 'G')?;
    cam.set(CAP_PROP_FOURCC, fourcc as f64)?;
    cam.set(CAP_PROP_FRAME_WIDTH, 1920.0)?;
    cam.set(CAP_PROP_FRAME_HEIGHT, 1080.0)?;
    cam.set(CAP_PROP_FPS, 30.0)?;

    let width = cam.get(CAP_PROP_FRAME_WIDTH)?;
    let height = cam.get(CAP_PROP_FRAME_HEIGHT)?;
    let fps = cam.get(CAP_PROP_FPS)?;
    log::info!("Camera resolution: {width}x{height}@{fps}fps");

    let _ = videoio::VideoCapture::is_opened(&cam)?;
    let mut ocr = LepTess::new(None, "eng")?;

    let mut frame = Mat::default();
    let mut gray = Mat::default();
    let mut edges = Mat::default();
    let mut rgba = Mat::default();
    let mut tiff = Vec::new();

    let mut frame_count = 0;
    let mut set_count: HashMap<String, u64> = HashMap::new();
    let mut number_count: HashMap<String, u64> = HashMap::new();

    loop {
        cam.read(&mut frame)?;

        let start = std::time::Instant::now();
        let mut contours = Vector::<Vector<Point>>::new();
        let mut set_contour = None;

        imgproc::cvt_color(
            &frame,
            &mut rgba,
            imgproc::COLOR_BGR2RGBA,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;
        imgproc::cvt_color(
            &frame,
            &mut gray,
            imgproc::COLOR_BGR2GRAY,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT,
        )?;
        imgproc::canny(&gray, &mut edges, 75.0, 200.0, 3, false)?;

        imgproc::find_contours(
            &edges,
            &mut contours,
            imgproc::RETR_EXTERNAL,
            imgproc::CHAIN_APPROX_SIMPLE,
            opencv::core::Point::new(0, 0),
        )?;
        imgproc::draw_contours(
            &mut rgba,
            &contours,
            -1,
            Scalar::new(0.0, 0.0, 255.0, 127.0),
            2,
            imgproc::LINE_8,
            &[],
            0,
            opencv::core::Point::new(0, 0),
        )?;

        for contour in contours {
            let perimeter = imgproc::arc_length(&contour, true)?;

            let mut approx = Vector::<Point>::new();
            imgproc::approx_poly_dp(&contour, &mut approx, 0.02 * perimeter, true)?;

            if approx.len() < 4 || !imgproc::is_contour_convex(&approx)? {
                continue;
            }

            let rect = imgproc::bounding_rect(&contour)?;
            let area = rect.width * rect.height;
            let aspect_ratio = rect.width as f32 / rect.height as f32;

            imgproc::rectangle(
                &mut rgba,
                rect,
                Scalar::new(0.0, 255.0, 0.0, 255.0),
                2,
                imgproc::LINE_8,
                0,
            )?;

            if !(1000..=10000).contains(&area) {
                continue;
            }

            if !(1.5..=5.0).contains(&aspect_ratio) {
                continue;
            }

            set_contour = Some(contour);
        }

        if let Some(set_contour) = set_contour {
            let set_region = imgproc::bounding_rect(&set_contour)?;

            imgproc::rectangle(
                &mut rgba,
                set_region,
                Scalar::new(255.0, 0.0, 0.0, 255.0),
                2,
                imgproc::LINE_8,
                0,
            )?;

            let set_id_region = Rect::new(
                (set_region.x as f32 * 1.015) as i32,
                (set_region.y as f32 * 1.008) as i32,
                (set_region.width as f32 - set_region.x as f32 * 0.02) as i32,
                (set_region.height as f32 - set_region.y as f32 * 0.015) as i32,
            );

            if let Ok(set_id) = Mat::roi(&gray, set_id_region) {
                let set_id = set_id.clone_pointee();

                if !set_id.is_continuous() {
                    continue;
                }

                overlay(&set_id, &mut rgba, 0, 0, 2)?;

                let size = set_id.size()?;
                let buf = set_id.data_bytes()?;

                let set_id =
                    image::GrayImage::from_raw(size.width as u32, size.height as u32, buf.to_vec())
                        .expect("Create image");

                set_id
                    .write_to(&mut io::Cursor::new(&mut tiff), image::ImageFormat::Tiff)
                    .unwrap();

                ocr.set_image_from_mem(&tiff).unwrap();
                ocr.set_source_resolution(70);
                ocr.set_variable(
                    leptess::Variable::TesseditCharWhitelist,
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZ ",
                )?;

                let set = ocr.get_utf8_text().unwrap();

                match set.split_whitespace().next().map(str::trim) {
                    Some(set) if !set.is_empty() => {
                        *set_count.entry(set.to_owned()).or_default() += 1;
                    }
                    _ => {}
                }
            }

            let number_region = Rect::new(
                set_region.x + set_region.width,
                set_region.y + set_region.height / 4,
                set_region.width * 2,
                set_region.height,
            );

            if let Ok(number) = Mat::roi(&gray, number_region) {
                let mut number = number.clone_pointee();

                if !number.is_continuous() {
                    continue;
                }

                let mut thresh = Mat::default();
                imgproc::threshold(&number, &mut thresh, 120.0, 255.0, imgproc::THRESH_BINARY)?;

                // Reverse holofoil
                if !is_mostly_black(&thresh)? {
                    imgproc::flood_fill(
                        &mut thresh,
                        Point::new(0, 0),
                        Scalar::new(255.0, 255.0, 255.0, 0.0),
                        &mut Rect::default(),
                        Scalar::new(10.0, 10.0, 10.0, 0.0),
                        Scalar::new(10.0, 10.0, 10.0, 0.0),
                        imgproc::FLOODFILL_FIXED_RANGE,
                    )?;

                    number = thresh;
                }

                overlay(&number, &mut rgba, 0, 100, 2)?;

                let size = number.size()?;
                let buf = number.data_bytes()?;

                let number =
                    image::GrayImage::from_raw(size.width as u32, size.height as u32, buf.to_vec())
                        .expect("Create image");

                number
                    .write_to(&mut io::Cursor::new(&mut tiff), image::ImageFormat::Tiff)
                    .unwrap();

                ocr.set_image_from_mem(&tiff).unwrap();
                ocr.set_source_resolution(70);
                ocr.set_variable(leptess::Variable::TesseditCharWhitelist, "0123456789/")?;

                let number = ocr.get_utf8_text().unwrap_or_default();

                match number.split('/').next().map(str::trim) {
                    Some(number)
                        if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) =>
                    {
                        *number_count.entry(number.to_owned()).or_default() += 1;
                    }
                    _ => {}
                }
            }
        }

        frame_count += 1;

        if frame_count >= 30 {
            let mut sets: Vec<_> = set_count.iter().collect();
            let mut numbers: Vec<_> = number_count.iter().collect();

            sets.sort_by_key(|(_, count)| *count);
            numbers.sort_by_key(|(_, count)| *count);

            if let Some((set, number)) = sets.last().zip(numbers.last()) {
                log::debug!("Detected: {} {}", set.0, number.0);

                sender.blocking_send(Event::Detected {
                    set: set.0.to_owned(),
                    number: number.0.to_owned(),
                })?;
            }

            frame_count = 0;
            set_count.clear();
            number_count.clear();
        }

        log::trace!("Frame processed in {:?}", start.elapsed());

        let size = rgba.size()?;

        sender.blocking_send(Event::Captured(Image {
            width: size.width as u32,
            height: size.height as u32,
            rgba: Bytes::copy_from_slice(rgba.data_bytes()?),
        }))?;
    }
}

fn is_mostly_black(thresh_img: &Mat) -> opencv::Result<bool> {
    let total = thresh_img.total();
    let non_black_pixels = opencv::core::count_non_zero(&thresh_img)?;
    let ratio = 1.0 - non_black_pixels as f32 / total as f32;

    Ok(ratio >= 0.9)
}

fn overlay(overlay: &Mat, background: &mut Mat, x: i32, y: i32, scale: i32) -> opencv::Result<()> {
    let mut resized = Mat::default();
    let mut rgba = Mat::default();

    imgproc::resize(
        overlay,
        &mut resized,
        overlay.size()? * scale,
        0.0,
        0.0,
        INTER_LINEAR,
    )?;
    imgproc::cvt_color(
        &resized,
        &mut rgba,
        imgproc::COLOR_BGR2RGBA,
        0,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let roi = Rect::new(x, y, rgba.cols(), rgba.rows());
    let mut target = Mat::roi_mut(background, roi)?;

    rgba.copy_to(&mut target)?;
    Ok(())
}
