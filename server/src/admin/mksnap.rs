/*
 * Created on Tue Oct 13 2020
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2020, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use crate::dbnet::connection::prelude::*;
use crate::kvengine::encoding;
use core::str;
use std::path::{Component, PathBuf};

action!(
    /// Create a snapshot
    ///
    fn mksnap(handle: &crate::corestore::Corestore, con: &mut T, mut act: ActionIter<'a>) {
        let engine = handle.get_engine();
        if act.len() == 0 {
            // traditional mksnap
            match engine.mksnap(handle.clone_store()).await {
                0 => conwrite!(con, groups::OKAY)?,
                1 => conwrite!(con, groups::SERVER_ERR)?,
                2 => conwrite!(con, groups::SNAPSHOT_DISABLED)?,
                3 => conwrite!(con, groups::SNAPSHOT_BUSY)?,
                _ => unsafe { impossible!() },
            }
        } else if act.len() == 1 {
            // remote snapshot, let's see what we've got
            let name = unsafe {
                // SAFETY: We have already checked that there is one item
                act.next_unchecked_bytes()
            };
            if !encoding::is_utf8(&name) {
                return conwrite!(con, groups::ENCODING_ERROR);
            }

            // SECURITY: Check for directory traversal syntax
            let st = unsafe {
                // SAFETY: We have already checked for UTF-8 validity
                str::from_utf8_unchecked(&name)
            };
            let path = PathBuf::from(st);
            let illegal_snapshot = path
                .components()
                .filter(|dir| {
                    // Sanitize snapshot name, to avoid directory traversal attacks
                    // If the snapshot name has any root directory or parent directory, then
                    // we'll allow it to pass through this adaptor.
                    // As a result, this iterator will give us a count of the 'bad' components
                    dir == &Component::RootDir || dir == &Component::ParentDir
                })
                .count()
                != 0;
            if illegal_snapshot {
                return conwrite!(con, groups::SNAPSHOT_ILLEGAL_NAME);
            }

            // now make the snapshot
            match engine.mkrsnap(name, handle.clone_store()).await {
                0 => conwrite!(con, groups::OKAY)?,
                1 => conwrite!(con, groups::SERVER_ERR)?,
                3 => conwrite!(con, groups::SNAPSHOT_BUSY)?,
                _ => unsafe { impossible!() },
            }
        } else {
            conwrite!(con, groups::ACTION_ERR)?;
        }
        Ok(())
    }
);
