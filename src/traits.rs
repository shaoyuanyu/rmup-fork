/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::path::Path;

use anyhow::Result;

pub trait Save {
    fn save<P: AsRef<Path>>(&self, file_path: P) -> Result<()>;
}

pub trait Load {
    fn load<P: AsRef<Path>>(file_path: P) -> Result<Self>
    where
        Self: Sized;
}
