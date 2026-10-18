#![forbid(unsafe_code)]

use nodex_state::StateTree;
use nodex_types::Address;
use wasmi::{Engine, Linker, Module, Store};

pub const MAX_WASM_CODE_BYTES: usize = 256 * 1024; // 256 KiB
pub const MAX_VM_MEMORY_PAGES: u32 = 256;         // 16 MiB

pub struct WasmVm;

impl WasmVm {
    pub fn validate_module(code: &[u8]) -> Result<(), String> {
        if code.len() > MAX_WASM_CODE_BYTES {
            return Err("WASM code exceeds maximum allowed size (256 KiB)".to_string());
        }

        let engine = Engine::default();
        let module = Module::new(&engine, code)
            .map_err(|e| format!("Invalid WASM module compilation: {}", e))?;

        // Inspect imports to ensure no non-ndx module imports
        for import in module.imports() {
            if import.module() != "ndx" {
                return Err(format!("Forbidden WASM import module: {}", import.module()));
            }
        }

        Ok(())
    }

    pub fn execute_contract(
        _state: &mut StateTree,
        _contract_address: &Address,
        code: &[u8],
        _method: &str,
        _args: &[u8],
        _fuel_limit: u64,
    ) -> Result<Vec<u8>, String> {
        Self::validate_module(code)?;

        let engine = Engine::default();
        let module = Module::new(&engine, code).map_err(|e| e.to_string())?;
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("WASM instantiation failed: {}", e))?
            .start(&mut store)
            .map_err(|e| format!("WASM start failed: {}", e))?;

        let _ = instance;
        Ok(vec![])
    }
}
