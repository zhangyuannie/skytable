/*
 * Created on Fri Jul 02 2021
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2021, Sayan Nandan <ohsayan@outlook.com>
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

//! # In-memory store
//!
//! This is what things look like:
//! ```text
//! ------------------------------------------------------
//! |                          |                         |
//! |  |-------------------|   |  |-------------------|  |
//! |  |-------------------|   |  |-------------------|  |
//! |  | | TABLE | TABLE | |   |  | | TABLE | TABLE | |  |
//! |  | |-------|-------| |   |  | |-------|-------| |  |
//! |  |      Keyspace     |   |  |      Keyspace     |  |
//! |  |-------------------|   |  |-------------------|  |
//!                            |                         |
//! |  |-------------------|   |  |-------------------|  |
//! |  | |-------|-------| |   |  | |-------|-------| |  |
//! |  | | TABLE | TABLE | |   |  | | TABLE | TABLE | |  |
//! |  | |-------|-------| |   |  | |-------|-------| |  |
//! |  |      Keyspace     |   |  |      Keyspace     |  |
//! |  |-------------------|   |  |-------------------|  |
//! |                          |                         |
//! |                          |                         |
//! |        NAMESPACE         |        NAMESPACE        |
//! ------------------------------------------------------
//! |                         NODE                       |
//! |----------------------------------------------------|
//! ```
//!
//! So, all your data is at the mercy of [`Memstore`]'s constructor
//! and destructor.

#![allow(dead_code)] // TODO(@ohsayan): Remove this onece we're done

use crate::coredb::htable::Coremap;
use crate::coredb::htable::Data;
use crate::coredb::SnapshotStatus;
use crate::kvengine::KVEngine;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// This is for the future where every node will be allocated a shard
#[derive(Debug)]
pub enum ClusterShardRange {
    SingleNode,
}

impl Default for ClusterShardRange {
    fn default() -> Self {
        Self::SingleNode
    }
}

/// This is for the future for determining the replication strategy
#[derive(Debug)]
pub enum ReplicationStrategy {
    /// Single node, no replica sets
    Default,
}

impl Default for ReplicationStrategy {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug)]
/// The core in-memory table
///
/// This in-memory table that houses all keyspaces and namespaces along with other node
/// properties. This is the structure that you should clone and send around connections
/// for connection-level control abilities over the namespace
pub struct Memstore {
    /// the namespaces
    namespaces: Arc<Coremap<Data, Arc<Namespace>>>,
}

impl Memstore {
    /// Create a new empty in-memory table with literally nothing in it
    pub fn new_empty() -> Self {
        Self {
            namespaces: Arc::new(Coremap::new()),
        }
    }
    /// Create a new in-memory table with the default namespace, keyspace and the default
    /// tables. So, whenever you're calling this, this is what you get:
    /// ```text
    /// YOURNODE: {
    ///     NAMESPACES: [
    ///         "default" : {
    ///             KEYSPACES: ["default", "_system"]
    ///         }
    ///     ]
    /// }
    /// ```
    ///
    /// When you connect a client without any information about the namespace you're planning to
    /// use, you'll be connected to `ns:default/ks:default`. The `ns:default/ks:_system` is not
    /// for you. It's for the system
    pub fn new_default() -> Self {
        Self {
            namespaces: {
                let n = Coremap::new();
                n.true_if_insert(Data::from("default"), Arc::new(Namespace::empty_default()));
                Arc::new(n)
            },
        }
    }
}

#[derive(Debug)]
/// Namespaces hold keyspaces
pub struct Namespace {
    /// the keyspaces stored in this namespace
    keyspaces: Coremap<Data, Arc<Keyspace>>,
    /// the shard range
    shard_range: ClusterShardRange,
}

impl Namespace {
    /// Create an empty namespace with no keyspaces
    pub fn empty() -> Self {
        Self {
            keyspaces: Coremap::new(),
            shard_range: ClusterShardRange::default(),
        }
    }
    /// Create an empty namespace with the default keyspace that has a table `default` and
    /// a table `system`
    pub fn empty_default() -> Self {
        Self {
            keyspaces: {
                let ks = Coremap::new();
                ks.true_if_insert(Data::from("default"), Arc::new(Keyspace::empty_default()));
                ks
            },
            shard_range: ClusterShardRange::default(),
        }
    }
    /// Get an atomic reference to a keyspace, if it exists
    pub fn get_keyspace_atomic_ref(&self, keyspace_idenitifer: Data) -> Option<Arc<Keyspace>> {
        self.keyspaces.get(&keyspace_idenitifer).map(|v| v.clone())
    }
}

// TODO(@ohsayan): Optimize the memory layouts of the UDFs to ensure that sharing is very cheap

#[derive(Debug)]
/// A keyspace houses all the other tables
pub struct Keyspace {
    /// the tables
    tables: Coremap<Data, Arc<Table>>,
    /// current state of the disk flush status. if this is true, we're safe to
    /// go ahead with writes
    flush_state_healthy: AtomicBool,
    /// the snapshot configuration for this namespace
    snap_config: Option<SnapshotStatus>,
    /// the replication strategy for this namespace
    replication_strategy: ReplicationStrategy,
}

impl Keyspace {
    /// Create a new empty keyspace with the default tables: a `default` table and a
    /// `system` table
    pub fn empty_default() -> Self {
        Self {
            tables: {
                let ht = Coremap::new();
                // add the default table
                ht.true_if_insert(
                    Data::from("default"),
                    Arc::new(Table::KV(KVEngine::default())),
                );
                // add the system table
                ht.true_if_insert(
                    Data::from("_system"),
                    Arc::new(Table::KV(KVEngine::default())),
                );
                ht
            },
            flush_state_healthy: AtomicBool::new(true),
            snap_config: None,
            replication_strategy: ReplicationStrategy::default(),
        }
    }
    /// Create a new empty keyspace with zero tables
    pub fn empty() -> Self {
        Self {
            tables: Coremap::new(),
            flush_state_healthy: AtomicBool::new(true),
            snap_config: None,
            replication_strategy: ReplicationStrategy::default(),
        }
    }
    /// Get an atomic reference to a table in this keyspace if it exists
    pub fn get_table_atomic_ref(&self, table_identifier: Data) -> Option<Arc<Table>> {
        self.tables.get(&table_identifier).map(|v| v.clone())
    }
}
// same 8 byte ptrs; any chance of optimizations?

#[derive(Debug)]
/// The underlying table type. This is the place for the other data models (soon!)
pub enum Table {
    /// a key/value store
    KV(KVEngine),
}
