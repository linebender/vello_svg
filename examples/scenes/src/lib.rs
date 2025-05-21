// Copyright 2022 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scenes

// The following lints are part of the Linebender standard set,
// but resolving them has been deferred for now.
// Feel free to send a PR that solves one or more of these.
#![allow(
    missing_debug_implementations,
    missing_docs,
    single_use_lifetimes,
    elided_lifetimes_in_paths,
    unused_macro_rules,
    unreachable_pub,
    clippy::use_self,
    clippy::shadow_unrelated,
    clippy::partial_pub_fields,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::missing_assert_message,
    clippy::wildcard_imports
)]

#[cfg(not(target_arch = "wasm32"))]
pub mod download;
mod simple_text;
mod svg;
mod test_scenes;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
#[cfg(not(target_arch = "wasm32"))]
use download::Download;
pub use simple_text::RobotoText;
pub use svg::{default_scene, scene_from_files};
pub use test_scenes::test_scenes;

use vello::Scene;
use vello::kurbo::Vec2;
use vello::peniko::{Color, color};

pub struct SceneParams<'a> {
    pub time: f64,
    /// Whether blocking should be limited
    /// Will not change between runs
    // TODO: Just never block/handle this automatically?
    pub interactive: bool,
    pub text: &'a mut RobotoText,
    pub resolution: Option<Vec2>,
    pub base_color: Option<Color>,
    pub complexity: usize,
}

pub struct SceneConfig {
    // TODO: This is currently unused
    pub animated: bool,
    pub name: String,
}

pub struct ExampleScene {
    pub function: Box<dyn TestScene>,
    pub config: SceneConfig,
}

pub trait TestScene {
    fn render(&mut self, scene: &mut Scene, params: &mut SceneParams);
}

impl<F: FnMut(&mut Scene, &mut SceneParams)> TestScene for F {
    fn render(&mut self, scene: &mut Scene, params: &mut SceneParams) {
        self(scene, params);
    }
}

pub struct SceneSet {
    pub scenes: Vec<ExampleScene>,
}

#[derive(Args, Debug)]
/// Shared config for scene selection
pub struct Arguments {
    #[arg(help_heading = "Scene Selection")]
    #[arg(long, global(false))]
    /// Whether to use the test scenes created by code
    test_scenes: bool,
    #[arg(help_heading = "Scene Selection", global(false))]
    /// The svg files paths to render
    svgs: Option<Vec<PathBuf>>,
    #[arg(help_heading = "Render Parameters")]
    #[arg(long, global(false), value_parser = parse_color)]
    /// The base color applied as the blend background to the rasterizer.
    /// Format is CSS style hexadecimal (#RGB, #RGBA, #RRGGBB, #RRGGBBAA) or
    /// an SVG color name such as "aliceblue"
    pub base_color: Option<Color>,
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Download SVG files for testing. By default, downloads a set of files from wikipedia
    #[cfg(not(target_arch = "wasm32"))]
    Download(Download),
}

impl Arguments {
    pub fn select_scene_set(
        &self,
        #[allow(unused)] command: impl FnOnce() -> clap::Command,
    ) -> Result<Option<SceneSet>> {
        if let Some(command) = &self.command {
            command.action()?;
            Ok(None)
        } else {
            // There is no file access on WASM, and on Android we haven't set up the assets
            // directory.
            // TODO: Upload the assets directory on Android
            // Therefore, only render the `test_scenes` (including one SVG example)
            #[cfg(any(target_arch = "wasm32", target_os = "android"))]
            return Ok(Some(test_scenes()));
            #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
            if self.test_scenes {
                Ok(test_scenes())
            } else if let Some(svgs) = &self.svgs {
                scene_from_files(svgs)
            } else {
                default_scene(command)
            }
            .map(Some)
        }
    }
}

impl Command {
    fn action(&self) -> Result<()> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Command::Download(download) => download.action(),
            #[cfg(target_arch = "wasm32")]
            _ => unreachable!("downloads not supported on wasm"),
        }
    }
}

fn parse_color(s: &str) -> Result<Color, color::ParseError> {
    color::parse_color(s).map(|c| c.to_alpha_color())
}
