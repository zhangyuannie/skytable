/*
 * Created on Wed Aug 19 2020
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

//! # `DEL` queries
//! This module provides functions to work with `DEL` queries

use crate::dbnet::connection::prelude::*;
use crate::util::compiler;

action!(
    /// Run a `DEL` query
    ///
    /// Do note that this function is blocking since it acquires a write lock.
    /// It will write an entire datagroup, for this `del` action
    fn del(handle: &Corestore, con: &'a mut T, act: ActionIter<'a>) {
        err_if_len_is!(act, con, eq 0);
        let kve = kve!(con, handle);
        let encoding_is_okay = if kve.needs_key_encoding() {
            true
        } else {
            let encoder = kve.get_key_encoder();
            act.as_ref().all(|k| encoder.is_ok(k))
        };
        if compiler::likely(encoding_is_okay) {
            let done_howmany: Option<usize>;
            {
                if registry::state_okay() {
                    let mut many = 0;
                    act.for_each(|key| {
                        if kve.remove_unchecked(key) {
                            many += 1
                        }
                    });
                    done_howmany = Some(many);
                } else {
                    done_howmany = None;
                }
            }
            if let Some(done_howmany) = done_howmany {
                con.write_response(done_howmany).await
            } else {
                con.write_response(responses::groups::SERVER_ERR).await
            }
        } else {
            compiler::cold_err(conwrite!(con, groups::ENCODING_ERROR))
        }
    }
);
