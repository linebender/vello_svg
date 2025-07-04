// Copyright 2023 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use notify_debouncer_mini::notify::*;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};

pub(crate) fn hot_reload(mut f: impl FnMut() -> Option<()> + Send + 'static) -> Result<impl Sized> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        None,
        move |res: DebounceEventResult| match res {
            Ok(_) => f().unwrap(),
            Err(errors) => errors.iter().for_each(|e| println!("Error {e:?}")),
        },
    )?;

    debouncer.watcher().watch(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../shader")
            .canonicalize()?,
        // We currently don't support hot reloading the imports, so don't
        // recurse into there
        RecursiveMode::NonRecursive,
    )?;
    Ok(debouncer)
}
