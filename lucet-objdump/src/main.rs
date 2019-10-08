#![deny(bare_trait_objects)]

use lucet_module::{
    FunctionSpec, Module, ModuleData, SerializedModule, TableElement, TrapManifest, TrapSite,
    VersionInfo,
};

use byteorder::{LittleEndian, ReadBytesExt};
use colored::Colorize;
use goblin::{elf, Object};
use std::env;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::mem;

#[derive(Debug)]
struct ArtifactSummary<'a> {
    buffer: &'a Vec<u8>,
    elf: &'a elf::Elf<'a>,
    symbols: StandardSymbols,
    data_segments: Option<DataSegments>,
    serialized_module: Option<SerializedModule>,
    exported_functions: Vec<&'a str>,
    imported_symbols: Vec<&'a str>,
}

#[derive(Debug)]
struct StandardSymbols {
    lucet_module: Option<elf::sym::Sym>,
}

#[derive(Debug)]
struct DataSegments {
    segments: Vec<DataSegment>,
}

#[derive(Debug)]
struct DataSegment {
    offset: u32,
    len: u32,
    data: Vec<u8>,
}

impl<'a> ArtifactSummary<'a> {
    fn new(buffer: &'a Vec<u8>, elf: &'a elf::Elf<'_>) -> Self {
        Self {
            buffer: buffer,
            elf: elf,
            symbols: StandardSymbols { lucet_module: None },
            data_segments: None,
            serialized_module: None,
            exported_functions: Vec::new(),
            imported_symbols: Vec::new(),
        }
    }

    fn read_memory(&self, addr: u64, size: u64) -> Option<&'a [u8]> {
        for header in &self.elf.program_headers {
            if header.p_type == elf::program_header::PT_LOAD {
                // Bounds check the entry
                if addr >= header.p_vaddr && (addr + size) <= (header.p_vaddr + header.p_memsz) {
                    let start = (addr - header.p_vaddr + header.p_offset) as usize;
                    let end = start + size as usize;

                    return Some(&self.buffer[start..end]);
                }
            }
        }

        None
    }

    fn gather(&mut self) {
        for ref sym in self.elf.syms.iter() {
            let name = self
                .elf
                .strtab
                .get(sym.st_name)
                .unwrap_or(Ok("(no name)"))
                .expect("strtab entry");

            match name {
                "lucet_module" => self.symbols.lucet_module = Some(sym.clone()),
                _ => {
                    if sym.st_bind() == elf::sym::STB_GLOBAL {
                        if sym.is_function() {
                            self.exported_functions.push(name.clone());
                        } else if sym.st_shndx == elf::section_header::SHN_UNDEF as usize {
                            self.imported_symbols.push(name.clone());
                        }
                    }
                }
            }
        }

        self.serialized_module = self.symbols.lucet_module.as_ref().map(|module_sym| {
            let buffer = self
                .read_memory(
                    module_sym.st_value,
                    mem::size_of::<SerializedModule>() as u64,
                )
                .unwrap();
            let mut rdr = Cursor::new(buffer);

            let version = VersionInfo::read_from(&mut rdr).unwrap();

            SerializedModule {
                version,
                module_data_ptr: rdr.read_u64::<LittleEndian>().unwrap(),
                module_data_len: rdr.read_u64::<LittleEndian>().unwrap(),
                tables_ptr: rdr.read_u64::<LittleEndian>().unwrap(),
                tables_len: rdr.read_u64::<LittleEndian>().unwrap(),
                function_manifest_ptr: rdr.read_u64::<LittleEndian>().unwrap(),
                function_manifest_len: rdr.read_u64::<LittleEndian>().unwrap(),
            }
        });
    }

    fn get_func_name_for_addr(&self, addr: u64) -> Option<&str> {
        for ref sym in self.elf.syms.iter() {
            if sym.is_function() && sym.st_value == addr {
                let name = self
                    .elf
                    .strtab
                    .get(sym.st_name)
                    .unwrap_or(Ok("(no name)"))
                    .expect("strtab entry");

                return Some(name);
            }
        }
        None
    }
}

fn main() {
    let path = env::args().nth(1).unwrap();
    let mut fd = File::open(path).expect("open");
    let mut buffer = Vec::new();
    fd.read_to_end(&mut buffer).expect("read");
    let object = Object::parse(&buffer).expect("parse");

    if let Object::Elf(eo) = object {
        let mut summary = ArtifactSummary::new(&buffer, &eo);
        summary.gather();
        print_summary(summary);
    } else {
        println!("Expected Elf!");
    }
}

/// Parse a trap manifest for function `f`, if it has one.
///
/// `parse_trap_manifest` may very understandably be confusing. Why not use `f.traps()`? In
/// `lucet-objdump` the module has been accessed by reading the file and following structures as
/// they exist at rest. This means pointers are not relocated, so slices that would be valid when
/// loaded through the platform's loader currently have pointers that are not valid for memory
/// access.
///
/// In particular, trap pointers are correct with respect to 0 being the start of the file (or,
/// buffer, after reading), which means we can (and must) rebuild a correct slice from the buffer.
fn parse_trap_manifest<'a>(
    summary: &'a ArtifactSummary<'a>,
    f: &FunctionSpec,
) -> Option<TrapManifest<'a>> {
    if let Some(faulty_trap_manifest) = f.traps() {
        let trap_ptr = faulty_trap_manifest.traps.as_ptr();
        let traps_count = faulty_trap_manifest.traps.len();
        let traps_byte_count = traps_count * std::mem::size_of::<TrapManifest<'_>>();
        if let Some(traps_byte_slice) =
            summary.read_memory(trap_ptr as u64, traps_byte_count as u64)
        {
            let real_trap_ptr = traps_byte_slice.as_ptr() as *const TrapSite;
            Some(TrapManifest {
                traps: unsafe { std::slice::from_raw_parts(real_trap_ptr, traps_count) },
            })
        } else {
            println!(
                "Failed to read trap bytes for function {:?}, at {:p}",
                f, trap_ptr
            );
            None
        }
    } else {
        None
    }
}

fn load_module<'b, 'a: 'b>(
    summary: &'a ArtifactSummary<'a>,
    serialized_module: &SerializedModule,
    tables: &'b [&[TableElement]],
) -> Module<'b> {
    let module_data_bytes = summary
        .read_memory(
            serialized_module.module_data_ptr,
            serialized_module.module_data_len,
        )
        .unwrap();

    let module_data =
        ModuleData::deserialize(module_data_bytes).expect("ModuleData can be deserialized");

    let function_manifest_bytes = summary
        .read_memory(
            serialized_module.function_manifest_ptr,
            serialized_module.function_manifest_len,
        )
        .unwrap();
    let function_manifest = unsafe {
        std::slice::from_raw_parts(
            function_manifest_bytes.as_ptr() as *const FunctionSpec,
            serialized_module.function_manifest_len as usize,
        )
    };
    Module {
        version: serialized_module.version.clone(),
        module_data,
        tables,
        function_manifest,
    }
}

fn summarize_module<'a, 'b: 'a>(summary: &'a ArtifactSummary<'a>, module: &Module<'b>) {
    let module_data = &module.module_data;
    let tables = module.tables;
    let function_manifest = module.function_manifest;

    println!("  Heap Specification:");
    if let Some(heap_spec) = module_data.heap_spec() {
        println!("  {:9}: {} bytes", "Reserved", heap_spec.reserved_size);
        println!("  {:9}: {} bytes", "Guard", heap_spec.guard_size);
        println!("  {:9}: {} bytes", "Initial", heap_spec.initial_size);
        if let Some(max_size) = heap_spec.max_size {
            println!("  {:9}: {} bytes", "Maximum", max_size);
        } else {
            println!("  {:9}: None", "Maximum");
        }
    } else {
        println!("  {}", "MISSING".red().bold());
    }

    println!("");
    println!("  Sparse Page Data:");
    if let Some(sparse_page_data) = module_data.sparse_data() {
        println!("  {:6}: {}", "Count", sparse_page_data.pages().len());
        let mut allempty = true;
        let mut anyempty = false;
        for (i, page) in sparse_page_data.pages().iter().enumerate() {
            match page {
                Some(page) => {
                    allempty = false;
                    println!(
                        "  Page[{}]: {:p}, size: {}",
                        i,
                        page.as_ptr(),
                        if page.len() != 4096 {
                            format!(
                                "{} (page size, expected 4096)",
                                format!("{}", page.len()).bold().red()
                            )
                            .red()
                        } else {
                            format!("{}", page.len()).green()
                        }
                    );
                }
                None => {
                    anyempty = true;
                }
            };
        }
        if allempty && sparse_page_data.pages().len() > 0 {
            println!("  (all pages empty)");
        } else if anyempty {
            println!("  (empty pages omitted)");
        }
    } else {
        println!("  {}", "MISSING!".red().bold());
    }

    println!("");
    println!("Tables:");
    if tables.len() == 0 {
        println!("  No tables.");
    } else {
        for (i, table) in tables.iter().enumerate() {
            println!("  Table {}: {:?}", i, table);
        }
    }

    println!("");
    println!("Signatures:");
    for (i, s) in module_data.signatures().iter().enumerate() {
        println!("  Signature {}: {}", i, s);
    }

    println!("");
    println!("Functions:");
    if function_manifest.len() != module_data.function_info().len() {
        println!(
            "    {} function manifest and function info have diverging function counts",
            "lucetc bug:".red().bold()
        );
        println!(
            "      function_manifest length   : {}",
            function_manifest.len()
        );
        println!(
            "      module data function count : {}",
            module_data.function_info().len()
        );
        println!("    Will attempt to display information about functions anyway, but trap/code information may be misaligned with symbols and signatures.");
    }

    for (i, f) in function_manifest.iter().enumerate() {
        let header_name = summary.get_func_name_for_addr(f.ptr().as_usize() as u64);

        if i >= module_data.function_info().len() {
            // This is one form of the above-mentioned bug case
            // Half the function information is missing, so just report the issue and continue.
            println!(
                "  Function {} {}",
                i,
                "is missing the module data part of its declaration".red()
            );
            match header_name {
                Some(name) => {
                    println!("    ELF header name: {}", name);
                }
                None => {
                    println!("    No corresponding ELF symbol.");
                }
            };
            break;
        }

        let colorize_name = |x: Option<&str>| match x {
            Some(name) => name.green(),
            None => "None".red().bold(),
        };

        let fn_meta = &module_data.function_info()[i];
        println!("  Function {} (name: {}):", i, colorize_name(fn_meta.name));
        if fn_meta.name != header_name {
            println!(
                "    Name {} with name declared in ELF headers: {}",
                "DISAGREES".red().bold(),
                colorize_name(header_name)
            );
        }

        println!(
            "    Signature (index {}): {}",
            fn_meta.signature.as_u32() as usize,
            module_data.signatures()[fn_meta.signature.as_u32() as usize]
        );

        println!("    Start: {:#010x}", f.ptr().as_usize());
        println!("    Code length: {} bytes", f.code_len());
        if let Some(trap_manifest) = parse_trap_manifest(&summary, f) {
            let trap_count = trap_manifest.traps.len();

            println!("    Trap information:");
            if trap_count > 0 {
                println!(
                    "      {} {} ...",
                    trap_manifest.traps.len(),
                    if trap_count == 1 { "trap" } else { "traps" },
                );
                for trap in trap_manifest.traps {
                    println!("        $+{:#06x}: {:?}", trap.offset, trap.code);
                }
            } else {
                println!("      No traps for this function");
            }
        }
    }

    println!("");
    println!("Globals:");
    if module_data.globals_spec().len() > 0 {
        for global_spec in module_data.globals_spec().iter() {
            println!("  {:?}", global_spec.global());
            for name in global_spec.export_names() {
                println!("    Exported as: {}", name);
            }
        }
    } else {
        println!("  None");
    }

    println!("");
    println!("Exported Functions/Symbols:");
    let mut exported_symbols = summary.exported_functions.clone();
    for export in module_data.export_functions() {
        match module_data.function_info()[export.fn_idx.as_u32() as usize].name {
            Some(name) => {
                println!("  Internal name: {}", name);

                // The "internal name" is probably the first exported name for this function.
                // Remove it from the exported_symbols list to not double-count
                if let Some(idx) = exported_symbols.iter().position(|x| *x == name) {
                    exported_symbols.remove(idx);
                }
            }
            None => {
                println!("  No internal name");
            }
        }

        // Export names do not have the guest_func_ prefix that symbol names get, and as such do
        // not need to be removed from `exported_symbols` (which is built entirely from
        // ELF-declared exports, with namespaced names)
        println!("    Exported as: {}", export.names.join(", "));
    }

    if exported_symbols.len() > 0 {
        println!("");
        println!("  Other exported symbols (from ELF headers):");
        for export in exported_symbols {
            println!("    {}", export);
        }
    }

    println!("");
    println!("Imported Functions/Symbols:");
    let mut imported_symbols = summary.imported_symbols.clone();
    for import in module_data.import_functions() {
        match module_data.function_info()[import.fn_idx.as_u32() as usize].name {
            Some(name) => {
                println!("  Internal name: {}", name);
            }
            None => {
                println!("  No internal name");
            }
        }
        println!("    Imported as: {}/{}", import.module, import.name);

        // Remove from the imported_symbols list to not double-count imported functions
        if let Some(idx) = imported_symbols.iter().position(|x| x == &import.name) {
            imported_symbols.remove(idx);
        }
    }

    if imported_symbols.len() > 0 {
        println!("");
        println!("  Other imported symbols (from ELF headers):");
        for import in &imported_symbols {
            println!("    {}", import);
        }
    }
}

fn print_summary(summary: ArtifactSummary<'_>) {
    println!("Required Symbols:");
    println!(
        "  {:30}: {}",
        "lucet_module",
        exists_to_str(&summary.symbols.lucet_module)
    );
    if let Some(ref serialized_module) = summary.serialized_module {
        println!("Native module components:");
        println!(
            "  {:30}: {}",
            "module_data_ptr",
            ptr_to_str(serialized_module.module_data_ptr)
        );
        println!(
            "  {:30}: {}",
            "module_data_len", serialized_module.module_data_len
        );
        println!(
            "  {:30}: {}",
            "tables_ptr",
            ptr_to_str(serialized_module.tables_ptr)
        );
        println!("  {:30}: {}", "tables_len", serialized_module.tables_len);
        println!(
            "  {:30}: {}",
            "function_manifest_ptr",
            ptr_to_str(serialized_module.function_manifest_ptr)
        );
        println!(
            "  {:30}: {}",
            "function_manifest_len", serialized_module.function_manifest_len
        );

        let tables_bytes = summary
            .read_memory(
                serialized_module.tables_ptr,
                serialized_module.tables_len * mem::size_of::<&[TableElement]>() as u64,
            )
            .unwrap();
        let tables = unsafe {
            std::slice::from_raw_parts(
                tables_bytes.as_ptr() as *const &[TableElement],
                serialized_module.tables_len as usize,
            )
        };
        let mut reconstructed_tables = Vec::new();
        // same situation as trap tables - these slices are valid as if the module was
        // dlopen'd, but we just read it as a flat file. So read through the ELF view and use
        // pointers to that for the real slices.

        for table in tables {
            let table_bytes = summary
                .read_memory(
                    table.as_ptr() as usize as u64,
                    (table.len() * mem::size_of::<TableElement>()) as u64,
                )
                .unwrap();
            reconstructed_tables.push(unsafe {
                std::slice::from_raw_parts(
                    table_bytes.as_ptr() as *const TableElement,
                    table.len() as usize,
                )
            });
        }

        let module = load_module(&summary, serialized_module, &reconstructed_tables);
        println!("\nModule:");
        summarize_module(&summary, &module);
    } else {
        println!("The symbol `lucet_module` is {}, so lucet-objdump cannot look at most of the interesting parts.", "MISSING".red().bold());
    }

    println!("");
    println!("Data Segments:");
    if let Some(data_segments) = summary.data_segments {
        println!("  {:6}: {}", "Count", data_segments.segments.len());
        for segment in &data_segments.segments {
            println!(
                "  {:7}: {:6}  {:6}: {:6}",
                "Offset", segment.offset, "Length", segment.len,
            );
        }
    } else {
        println!("  {}", "MISSING!".red().bold());
    }
}

fn ptr_to_str(p: u64) -> colored::ColoredString {
    if p != 0 {
        format!("exists; address: {:#x}", p).green()
    } else {
        "MISSING!".red().bold()
    }
}

fn exists_to_str<T>(p: &Option<T>) -> colored::ColoredString {
    return match p {
        Some(_) => "exists".green(),
        None => "MISSING!".red().bold(),
    };
}
