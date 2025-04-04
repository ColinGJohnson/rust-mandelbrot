use crate::Args;
use indicatif::ProgressBar;
use num::Complex;
use num::integer::Roots;
use rand::Rng;
use rayon::prelude::*;

#[derive(Copy, Clone)]
struct Pixel {
    x: u32,
    y: u32,
}

pub struct SampleResult {
    pub x_res: u32,
    pub y_res: u32,
    pub grid: Vec<Vec<Option<f64>>>,
}

pub fn sample_grid(args: &Args, progress_bar: &ProgressBar) -> SampleResult {
    let offset = Complex::new(args.real_offset, args.complex_offset);
    let center = (Complex::new(args.x_res as f64, args.y_res as f64) / args.zoom) / 2f64;
    let mut result = vec![vec![None; args.y_res as usize]; args.x_res as usize];

    // TODO: Degree of parallelism shouldn't depend on the image size & aspect ratio
    result.par_iter_mut().enumerate().for_each(|(x, column)| {
        for y in 0..args.y_res {
            let sample = sample_pixel(args, offset, center, x as u32, y);
            column[y as usize] = sample;
            if y % 100 == 0 {
                progress_bar.inc(100);
            }
        }
    });

    SampleResult {
        x_res: args.x_res,
        y_res: args.y_res,
        grid: result,
    }
}

fn sample_pixel(args: &Args, offset: Complex<f64>, center: Complex<f64>, x: u32, y: u32) -> Option<f64> {
    let location: Complex<f64> = pixel_to_complex(Pixel { x, y }, center, offset, args.zoom);
    super_sample_mandelbrot(args, location)
}

/// Convert a pixel location to a location on the complex plane.
fn pixel_to_complex(location: Pixel, center: Complex<f64>, offset: Complex<f64>, zoom: f64) -> Complex<f64> {
    let sample = Complex::new(location.x as f64, location.y as f64) / zoom;
    sample + offset - center
}

/// Returns the average of multiple samples within a given range. Sampling uses a "jitter" strategy.
/// https://en.wikipedia.org/wiki/Supersampling.
fn super_sample_mandelbrot(args: &Args, c: Complex<f64>) -> Option<f64> {
    let mut sum = 0f64;
    let mut diverged_samples = 0;
    let subpixel_width = (1.0 / args.zoom) / (args.samples as f64).sqrt();

    for dx in 0..args.samples.sqrt() {
        for dy in 0..args.samples.sqrt() {
            let subpixel_center = c + Complex::new(
                dx as f64 * subpixel_width, dy as f64 * subpixel_width);
            let sample_location = random_offset(subpixel_center, subpixel_width);
            if let Some(sample) = sample_mandelbrot(args, sample_location) {
                sum += sample;
                diverged_samples += 1
            }
        }
    }

    if diverged_samples > 0 {
        Some(sum / diverged_samples as f64)
    } else {
        None
    }
}

fn random_offset(c: Complex<f64>, range: f64) -> Complex<f64> {
    let mut rng = rand::rng();
    let half_range = range / 2.0;
    let re = rng.random_range(-half_range..half_range);
    let im = rng.random_range(-half_range..half_range);
    c + Complex::new(re, im)
}

/// Sample the mandelbrot set at the given location.
/// Returns num iterations before the sequence diverged, or None if the sequence did not diverge.
fn sample_mandelbrot(args: &Args, c: Complex<f64>) -> Option<f64> {
    let mut z = Complex::new(0.0, 0.0);
    for iteration in 0..args.max_iterations {
        z = (z * z) + c;
        if f64::hypot(z.re, z.im) > args.threshold {
            return if args.smooth {
                Some(smooth_iteration(iteration, z))
            } else {
                Some((iteration + 1) as f64)
            }
        }
    }
    None
}

/// Calculates a "smooth" escape time to improve color gradients in the rendered Mandelbrot set.
///
/// # Parameters
/// - `iteration`: The number of iterations before the sequence diverged.
/// - `z`: The final value of the `z` term when the sequence diverged.
fn smooth_iteration(iteration: u32, z: Complex<f64>) -> f64 {
    iteration as f64 + 1.0 - ((z.norm().ln() / 2.0_f64.ln()).ln() / 2.0_f64.ln())
}
