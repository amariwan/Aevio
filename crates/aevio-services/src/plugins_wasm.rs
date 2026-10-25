//! Isolated WebAssembly execution for executable plugin components.
//!
//! The ABI is intentionally small and explicit. A component exports:
//! `memory`, `aevio_alloc(i32) -> i32`, and
//! `aevio_handle(i32, i32, i32, i32) -> i32`. The host supplies one import,
//! `aevio_host.call`, and rejects every other import. JSON crosses the
//! boundary as bounded UTF-8; the guest cannot access WASI, the filesystem,
//! the network, or native process APIs.

use std::path::Path;

use serde_json::Value;
use wasmtime::{Caller, Config, Engine, Linker, Module, ResourceLimiter, Store};

const PAGE_BYTES: usize = 64 * 1024;
const MAX_MEMORY_BYTES: usize = 16 * 1024 * 1024;
const MAX_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_FUEL: u64 = 5_000_000;

#[derive(Debug, Clone, Copy)]
pub struct WasmLimits {
    pub max_memory_bytes: usize,
    pub fuel: u64,
}

impl Default for WasmLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: MAX_MEMORY_BYTES,
            fuel: MAX_FUEL,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmPluginError {
    InvalidModule(String),
    MissingExport(&'static str),
    InvalidAbi(String),
    MessageTooLarge,
    InvalidMessage(String),
    HostCall(String),
    Trap(String),
    ResourceLimit(String),
}

impl std::fmt::Display for WasmPluginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidModule(error) => write!(formatter, "invalid plugin module: {error}"),
            Self::MissingExport(export) => {
                write!(formatter, "plugin module misses export {export}")
            }
            Self::InvalidAbi(error) => write!(formatter, "invalid plugin ABI: {error}"),
            Self::MessageTooLarge => write!(formatter, "plugin message exceeds host limit"),
            Self::InvalidMessage(error) => write!(formatter, "invalid plugin message: {error}"),
            Self::HostCall(error) => write!(formatter, "plugin host call failed: {error}"),
            Self::Trap(error) => write!(formatter, "plugin execution trapped: {error}"),
            Self::ResourceLimit(error) => {
                write!(formatter, "plugin resource limit exceeded: {error}")
            }
        }
    }
}

impl std::error::Error for WasmPluginError {}

type HostCall = dyn FnMut(&str, &Value) -> Result<Value, String>;

struct HostState {
    limits: Limits,
    host_call: Box<HostCall>,
}

struct Limits {
    max_memory_bytes: usize,
    memory_bytes: usize,
}

impl ResourceLimiter for Limits {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let maximum = maximum.unwrap_or(self.max_memory_bytes);
        if desired > self.max_memory_bytes || desired > maximum {
            return Ok(false);
        }
        self.memory_bytes = desired;
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(maximum.is_none_or(|maximum| desired <= maximum))
    }
}

pub fn validate_module(bytes: &[u8], limits: WasmLimits) -> Result<(), WasmPluginError> {
    let engine = engine(limits)?;
    let module = Module::new(&engine, bytes)
        .map_err(|error| WasmPluginError::InvalidModule(error.to_string()))?;
    validate_exports(&module)
}

pub fn validate_module_file(path: &Path, limits: WasmLimits) -> Result<(), WasmPluginError> {
    let bytes =
        std::fs::read(path).map_err(|error| WasmPluginError::InvalidModule(error.to_string()))?;
    validate_module(&bytes, limits)
}

pub fn execute(
    bytes: &[u8],
    request: &Value,
    limits: WasmLimits,
    host_call: impl FnMut(&str, &Value) -> Result<Value, String> + 'static,
) -> Result<Value, WasmPluginError> {
    let input = serde_json::to_vec(request)
        .map_err(|error| WasmPluginError::InvalidMessage(error.to_string()))?;
    if input.len() > MAX_MESSAGE_BYTES {
        return Err(WasmPluginError::MessageTooLarge);
    }
    let engine = engine(limits)?;
    let module = Module::new(&engine, bytes)
        .map_err(|error| WasmPluginError::InvalidModule(error.to_string()))?;
    validate_exports(&module)?;

    let mut linker = Linker::new(&engine);
    linker
        .func_wrap(
            "aevio_host",
            "call",
            |mut caller: Caller<'_, HostState>,
             request_ptr: i32,
             request_len: i32,
             response_ptr: i32,
             response_capacity: i32|
             -> wasmtime::Result<i32> {
                let request = read_json(&mut caller, request_ptr, request_len)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                let method = request
                    .get("method")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("host call is missing method"))?;
                let payload = request.get("payload").unwrap_or(&Value::Null);
                let result = (caller.data_mut().host_call)(method, payload)
                    .map_err(|error| anyhow::anyhow!(error))?;
                let bytes = serde_json::to_vec(&result)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                if bytes.len() > MAX_MESSAGE_BYTES
                    || bytes.len() > response_capacity.max(0) as usize
                {
                    return Err(anyhow::anyhow!("host response exceeds plugin buffer"));
                }
                let memory = caller
                    .get_export("memory")
                    .and_then(|export| export.into_memory())
                    .ok_or_else(|| anyhow::anyhow!("plugin memory export is unavailable"))?;
                memory
                    .write(&mut caller, response_ptr.max(0) as usize, &bytes)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                Ok(bytes.len() as i32)
            },
        )
        .map_err(|error| WasmPluginError::InvalidAbi(error.to_string()))?;

    let mut store = Store::new(
        &engine,
        HostState {
            limits: Limits {
                max_memory_bytes: limits.max_memory_bytes,
                memory_bytes: 0,
            },
            host_call: Box::new(host_call),
        },
    );
    store.limiter(|state| &mut state.limits);
    store.set_epoch_deadline(1);
    store
        .set_fuel(limits.fuel)
        .map_err(|error| WasmPluginError::ResourceLimit(error.to_string()))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|error| WasmPluginError::Trap(error.to_string()))?;
    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or(WasmPluginError::MissingExport("memory"))?;
    let alloc = instance
        .get_typed_func::<i32, i32>(&mut store, "aevio_alloc")
        .map_err(|error| WasmPluginError::InvalidAbi(error.to_string()))?;
    let handle = instance
        .get_typed_func::<(i32, i32, i32, i32), i32>(&mut store, "aevio_handle")
        .map_err(|error| WasmPluginError::InvalidAbi(error.to_string()))?;
    let input_ptr = alloc
        .call(&mut store, input.len() as i32)
        .map_err(|error| classify_trap(error.to_string()))?;
    let output_capacity = MAX_MESSAGE_BYTES.min(limits.max_memory_bytes / 2);
    let output_ptr = alloc
        .call(&mut store, output_capacity as i32)
        .map_err(|error| classify_trap(error.to_string()))?;
    memory
        .write(&mut store, input_ptr.max(0) as usize, &input)
        .map_err(|error| WasmPluginError::Trap(error.to_string()))?;
    let output_len = handle
        .call(
            &mut store,
            (
                input_ptr,
                input.len() as i32,
                output_ptr,
                output_capacity as i32,
            ),
        )
        .map_err(|error| classify_trap(error.to_string()))?;
    if output_len < 0 || output_len as usize > output_capacity {
        return Err(WasmPluginError::InvalidMessage(
            "plugin returned an invalid output length".into(),
        ));
    }
    let mut output = vec![0; output_len as usize];
    memory
        .read(&store, output_ptr.max(0) as usize, &mut output)
        .map_err(|error| WasmPluginError::Trap(error.to_string()))?;
    serde_json::from_slice(&output)
        .map_err(|error| WasmPluginError::InvalidMessage(error.to_string()))
}

fn engine(limits: WasmLimits) -> Result<Engine, WasmPluginError> {
    if limits.max_memory_bytes == 0
        || !limits.max_memory_bytes.is_multiple_of(PAGE_BYTES)
        || limits.fuel == 0
    {
        return Err(WasmPluginError::ResourceLimit(
            "invalid runtime limits".into(),
        ));
    }
    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);
    Engine::new(&config).map_err(|error| WasmPluginError::InvalidModule(error.to_string()))
}

fn validate_exports(module: &Module) -> Result<(), WasmPluginError> {
    for import in module.imports() {
        if import.module() != "aevio_host" || import.name() != "call" {
            return Err(WasmPluginError::InvalidAbi(format!(
                "unsupported import {}.{}",
                import.module(),
                import.name()
            )));
        }
    }
    if !matches!(
        module.get_export("memory"),
        Some(wasmtime::ExternType::Memory(_))
    ) {
        return Err(WasmPluginError::MissingExport("memory"));
    }
    if module.get_export("aevio_alloc").is_none() {
        return Err(WasmPluginError::MissingExport("aevio_alloc"));
    }
    if module.get_export("aevio_handle").is_none() {
        return Err(WasmPluginError::MissingExport("aevio_handle"));
    }
    Ok(())
}

fn read_json(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<Value, WasmPluginError> {
    if ptr < 0 || len < 0 || len as usize > MAX_MESSAGE_BYTES {
        return Err(WasmPluginError::MessageTooLarge);
    }
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or(WasmPluginError::MissingExport("memory"))?;
    let mut bytes = vec![0; len as usize];
    memory
        .read(&mut *caller, ptr as usize, &mut bytes)
        .map_err(|error| WasmPluginError::InvalidMessage(error.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| WasmPluginError::InvalidMessage(error.to_string()))
}

fn classify_trap(error: String) -> WasmPluginError {
    if error.contains("fuel") || error.contains("memory") {
        WasmPluginError::ResourceLimit(error)
    } else {
        WasmPluginError::Trap(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module() -> Vec<u8> {
        wat::parse_str(
            r#"(module
                (import "aevio_host" "call" (func $call (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1 256)
                (global $heap (mut i32) (i32.const 1024))
                (func (export "aevio_alloc") (param $size i32) (result i32)
                    (local $ptr i32)
                    global.get $heap
                    local.tee $ptr
                    local.get $size
                    i32.add
                    global.set $heap
                    local.get $ptr)
                (func (export "aevio_handle") (param $ptr i32) (param $len i32) (param $out i32) (param $cap i32) (result i32)
                    local.get $ptr
                    local.get $len
                    local.get $out
                    local.get $cap
                    call $call))"#,
        )
        .unwrap()
    }

    #[test]
    fn executes_only_through_the_declared_host_call() {
        let result = execute(
            &module(),
            &serde_json::json!({"method":"echo","payload":{"ok":true}}),
            WasmLimits::default(),
            |method, payload| {
                assert_eq!(method, "echo");
                Ok(payload.clone())
            },
        )
        .unwrap();
        assert_eq!(result, serde_json::json!({"ok":true}));
    }

    #[test]
    fn rejects_modules_with_missing_abi() {
        let module = wat::parse_str("(module (memory (export \"memory\") 1))").unwrap();
        assert_eq!(
            validate_module(&module, WasmLimits::default()),
            Err(WasmPluginError::MissingExport("aevio_alloc"))
        );
    }
}
