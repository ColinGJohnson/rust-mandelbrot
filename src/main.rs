use std::time::Instant;
use clap::Parser;
use num::Complex;
use image::{Rgb, RgbImage};
use indicatif::{ProgressBar, ProgressStyle};
use show_image::{create_window, event};
use rayon::prelude::*;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output file path to use instead of the Image preview window.
    #[arg(short, long)]
    output: Option<String>,

    /// Width of the generated image.
    #[arg(short, long, default_value_t = 1000)]
    x_res: u32,

    /// Height of the generated image.
    #[arg(short, long, default_value_t = 1000)]
    y_res: u32,

    /// Center location on the real axis.
    #[arg(short, long, default_value_t = -1.0)]
    real_offset: f64,

    /// Center location on the imaginary axis.
    #[arg(short, long, default_value_t = 0.0)]
    complex_offset: f64,

    /// Zoom factor (pixels per unit distance on complex plane).
    #[arg(short, long, default_value_t = 250.0)]
    zoom: f64,

    /// Threshold past width the sequence is assumed to diverge.
    #[arg(short, long, default_value_t = 2.0)]
    threshold: f64,

    /// Number of iterations before assuming sequence does not diverge.
    #[arg(short, long, default_value_t = 100)]
    max_iterations: u32,

    /// Number of worker threads to run the calculation on.
    #[arg(short, long, default_value_t = 1)]
    workers: usize,
}

#[derive(Copy, Clone)]
struct PixelLocation {
    x: u32,
    y: u32,
}

#[show_image::main]
fn main() {
    let now = Instant::now();
    let args = Args::parse();
    let offset = Complex::new(args.real_offset, args.complex_offset);
    let center = Complex::new(args.x_res as f64, args.y_res as f64) / args.zoom / 2f64;

    let progress_bar = build_progress_bar((args.x_res * args.y_res) as u64);
    progress_bar.set_message("Sampling Mandelbrot");

    let mut image = RgbImage::new(args.x_res, args.y_res);
    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(args.workers).build().unwrap();
    thread_pool.install(|| {
        image.enumerate_pixels_mut().par_bridge().for_each(|(x, y, pixel)| {
            let complex_location: Complex<f64> = pixel_to_complex(PixelLocation { x, y }, center, offset, args.zoom);
            let color = match sample_mandelbrot(complex_location, args.threshold, args.max_iterations) {
                Some(iterations) => iterations_to_color(iterations, args.max_iterations),
                None => Rgb([0, 0, 0])
            };
            *pixel = color;
            progress_bar.inc(1);
        });
    });

    match args.output {
        Some(output) => {
            progress_bar.set_message("Saving image");
            image.save(output).unwrap();
        },
        None => {
            progress_bar.set_message("Displaying image");
            show_image(image).unwrap()
        }
    };

    progress_bar.set_message("Saving image");
    progress_bar.finish();

    let elapsed = now.elapsed().as_millis();
    println!("Finished in {elapsed}ms")
}

/// Display the image in a window and wait for the user to press escape.
fn show_image(image: RgbImage)-> Result<(), Box<dyn std::error::Error>> {
    let window = create_window("Mandelbrot", Default::default())?;
    window.set_image("Mandelbrot", image)?;
    for event in window.event_channel()? {
        if let event::WindowEvent::KeyboardInput(event) = event {
            if event.input.key_code == Some(event::VirtualKeyCode::Escape)
                && event.input.state.is_pressed() {
                break;
            }
        }
    }
    Ok(())
}

/// Construct a progress bar with a custom style.
fn build_progress_bar(len: u64) -> ProgressBar {
    let progress_bar = ProgressBar::new(len);
    progress_bar.set_style(
        ProgressStyle::with_template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len}")
            .unwrap()
            .progress_chars("#>-")
    );
    progress_bar
}

/// Convert a pixel location to a location on the complex plane.
fn pixel_to_complex( location: PixelLocation, center: Complex<f64>, offset: Complex<f64>, zoom: f64) -> Complex<f64> {
    let sample = Complex::new(location.x as f64, location.y as f64) / zoom;
    sample + offset - center
}

/// Map the number of iterations to a color.
fn iterations_to_color(iterations: u32, max_iterations: u32) -> Rgb<u8> {
    let t = iterations as f64 / max_iterations as f64;
    let color = ((1.0 - (t)) * 255.0) as u8;
    Rgb([color, color, color])
}

/// Sample the mandelbrot set at the given location.
/// Returns num iterations before the sequence diverged, or None if the sequence did not diverge.
fn sample_mandelbrot(c: Complex<f64>, threshold: f64, iterations: u32) -> Option<u32> {
    let threshold_squared = threshold * threshold;
    let mut z = Complex::new(0.0, 0.0);
    for iteration in 0..iterations {
        z = (z * z) + c;
        if z.norm_sqr() > threshold_squared {
            return Some(iteration + 1)
        }
    }
    return None;
}
