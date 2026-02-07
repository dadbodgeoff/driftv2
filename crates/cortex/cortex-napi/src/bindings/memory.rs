//! Memory CRUD bindings: create, get, update, delete, search, list, archive, restore.

use napi_derive::napi;

use cortex_core::traits::IMemoryStorage;

use crate::conversions::{error_types, memory_types};
use crate::runtime;

/// Create a new memory.
#[napi]
pub fn cortex_memory_create(memory_json: serde_json::Value) -> napi::Result<()> {
    let rt = runtime::get()?;
    let memory = memory_types::memory_from_json(memory_json)?;
    rt.storage
        .create(&memory)
        .map_err(error_types::to_napi_error)
}

/// Get a memory by ID.
#[napi]
pub fn cortex_memory_get(id: String) -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let memory = rt
        .storage
        .get(&id)
        .map_err(error_types::to_napi_error)?
        .ok_or_else(|| {
            error_types::to_napi_error(cortex_core::CortexError::MemoryNotFound { id })
        })?;
    memory_types::memory_to_json(&memory)
}

/// Update an existing memory.
#[napi]
pub fn cortex_memory_update(memory_json: serde_json::Value) -> napi::Result<()> {
    let rt = runtime::get()?;
    let memory = memory_types::memory_from_json(memory_json)?;
    rt.storage
        .update(&memory)
        .map_err(error_types::to_napi_error)
}

/// Delete a memory by ID.
#[napi]
pub fn cortex_memory_delete(id: String) -> napi::Result<()> {
    let rt = runtime::get()?;
    rt.storage.delete(&id).map_err(error_types::to_napi_error)
}

/// Full-text search for memories.
#[napi]
pub fn cortex_memory_search(query: String, limit: Option<i64>) -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let limit = limit.unwrap_or(20) as usize;
    let results = rt
        .storage
        .search_fts5(&query, limit)
        .map_err(error_types::to_napi_error)?;
    memory_types::memories_to_json(&results)
}

/// List memories by type.
#[napi]
pub fn cortex_memory_list(memory_type: Option<String>) -> napi::Result<serde_json::Value> {
    let rt = runtime::get()?;
    let memories = match memory_type {
        Some(type_str) => {
            let mt = memory_types::memory_type_from_string(&type_str)?;
            rt.storage
                .query_by_type(mt)
                .map_err(error_types::to_napi_error)?
        }
        None => {
            // Return all types
            let mut all = Vec::new();
            for mt in cortex_core::MemoryType::ALL {
                let batch = rt
                    .storage
                    .query_by_type(mt)
                    .map_err(error_types::to_napi_error)?;
                all.extend(batch);
            }
            all
        }
    };
    memory_types::memories_to_json(&memories)
}

/// Archive a memory (set archived = true).
#[napi]
pub fn cortex_memory_archive(id: String) -> napi::Result<()> {
    let rt = runtime::get()?;
    let mut memory = rt
        .storage
        .get(&id)
        .map_err(error_types::to_napi_error)?
        .ok_or_else(|| {
            error_types::to_napi_error(cortex_core::CortexError::MemoryNotFound { id: id.clone() })
        })?;
    memory.archived = true;
    rt.storage
        .update(&memory)
        .map_err(error_types::to_napi_error)
}

/// Restore an archived memory (set archived = false).
#[napi]
pub fn cortex_memory_restore(id: String) -> napi::Result<()> {
    let rt = runtime::get()?;
    let mut memory = rt
        .storage
        .get(&id)
        .map_err(error_types::to_napi_error)?
        .ok_or_else(|| {
            error_types::to_napi_error(cortex_core::CortexError::MemoryNotFound { id: id.clone() })
        })?;
    memory.archived = false;
    rt.storage
        .update(&memory)
        .map_err(error_types::to_napi_error)
}
