//! Embedded image device skins for all supported models.
//!
//! Each model's skin bitmap lives in its own module so replacing one
//! model's photograph cannot affect another. The shared decode, resize,
//! and composite pipeline stays here.

use anyhow::{Context, Result};
use image::imageops::FilterType;
use wqxemu_core::MachineModel;

use super::{SOURCE_HEIGHT, SOURCE_WIDTH};

mod cc800;
mod nc1020;
mod nc2000;
mod nc3000;
mod pc1000;

pub(super) fn render(model: MachineModel, width: usize, height: usize) -> Result<Vec<u32>> {
    let bytes: &[u8] = match model {
        MachineModel::Nc1020 => nc1020::SKIN,
        MachineModel::Nc2000 => nc2000::SKIN,
        MachineModel::Nc3000 => nc3000::SKIN,
        MachineModel::Pc1000 => pc1000::SKIN,
        MachineModel::Cc800 => cc800::SKIN,
    };
    let image = image::load_from_memory(bytes)
        .with_context(|| format!("failed to decode the {} device skin", model.name()))?
        .to_rgba8();
    anyhow::ensure!(
        image.width() as usize == SOURCE_WIDTH && image.height() as usize == SOURCE_HEIGHT,
        "unexpected {} skin size: {}x{}",
        model.name(),
        image.width(),
        image.height()
    );

    let image = image::imageops::resize(&image, width as u32, height as u32, FilterType::Lanczos3);
    let mut pixels = Vec::with_capacity(width * height);
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        // The source PNG is transparent outside the device outline.
        // Composite it over the normal light window background.
        let alpha = a as u32;
        let blend = |channel: u8| (channel as u32 * alpha + 0xF0 * (255 - alpha) + 127) / 255;
        pixels.push((blend(r) << 16) | (blend(g) << 8) | blend(b));
    }

    Ok(pixels)
}
