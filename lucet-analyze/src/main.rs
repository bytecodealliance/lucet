use byteorder::{LittleEndian, ReadBytesExt};
use colored::Colorize;
use goblin::{elf, Object};
use std::env;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::mem::size_of;

#[derive(Debug)]
struct ArtifactSummary<'a> {
    buffer: &'a Vec<u8>,
    elf: &'a elf::Elf<'a>,
    symbols: StandardSymbols,
    heap_spec: Option<HeapSpec>,
    globals_spec: Option<GlobalsSpec>,
    data_segments: Option<DataSegments>,
    sparse_page_data: Option<SparsePageData>,
    trap_manifest: Option<TrapManifest>,
    exported_functions: Vec<&'a str>,
    imported_symbols: Vec<&'a str>,
}

#[derive(Debug)]
struct StandardSymbols {
    lucet_trap_manifest: Option<elf::sym::Sym>,
    lucet_trap_manifest_len: Option<elf::sym::Sym>,
    wasm_data_segments: Option<elf::sym::Sym>,
    wasm_data_segments_len: Option<elf::sym::Sym>,
    lucet_heap_spec: Option<elf::sym::Sym>,
    lucet_globals_spec: Option<elf::sym::Sym>,
    guest_sparse_page_data: Option<elf::sym::Sym>,
}

#[derive(Debug)]
struct TrapManifest {
    records: Vec<TrapManifestRow>,
}

#[derive(Debug)]
struct TrapManifestRow {
    func_name: String,
    func_addr: u64,
    func_len: u64,
    trap_count: u64,
    sites: Vec<TrapSite>,
}

#[derive(Debug)]
struct TrapSite {
    offset: u32,
    trapcode: u32,
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

#[derive(Debug)]
struct SparsePageData {
    pages: Vec<*const u8>,
}

#[derive(Debug)]
struct HeapSpec {
    reserved_size: u64,
    guard_size: u64,
    initial_size: u64,
    max_size: Option<u64>,
}

#[derive(Debug)]
struct GlobalsSpec {
    count: u64,
}

impl<'a> ArtifactSummary<'a> {
    fn new(buffer: &'a Vec<u8>, elf: &'a elf::Elf) -> Self {
        Self {
            buffer: buffer,
            elf: elf,
            symbols: StandardSymbols {
                lucet_trap_manifest: None,
                lucet_trap_manifest_len: None,
                wasm_data_segments: None,
                wasm_data_segments_len: None,
                lucet_heap_spec: None,
                lucet_globals_spec: None,
                guest_sparse_page_data: None,
            },
            heap_spec: None,
            globals_spec: None,
            data_segments: None,
            sparse_page_data: None,
            trap_manifest: None,
            exported_functions: Vec::new(),
            imported_symbols: Vec::new(),
        }
    }

    fn read_memory(&self, addr: u64, size: u64) -> Option<Vec<u8>> {
        for header in &self.elf.program_headers {
            if header.p_type == elf::program_header::PT_LOAD {
                // Bounds check the entry
                if addr >= header.p_vaddr && (addr + size) < (header.p_vaddr + header.p_memsz) {
                    let start = (addr - header.p_vaddr + header.p_offset) as usize;
                    let end = start + size as usize;

                    return Some(self.buffer[start..end].to_vec());
                }
            }
        }

        None
    }

    fn gather(&mut self) {
        // println!("Syms");
        // for sym in eo.syms.iter() {
        //     let name = eo.strtab
        //         .get(sym.st_name)
        //         .unwrap_or(Ok("(no name)"))
        //         .expect("strtab entry");

        //     println!("Sym: name={} {:?}", name, sym);
        // }

        // println!("Dyn syms");

        for ref sym in self.elf.syms.iter() {
            let name = self
                .elf
                .strtab
                .get(sym.st_name)
                .unwrap_or(Ok("(no name)"))
                .expect("strtab entry");

            //println!("sym: name={} {:?}", name, sym);
            match name {
                "lucet_trap_manifest" => self.symbols.lucet_trap_manifest = Some(sym.clone()),
                "lucet_trap_manifest_len" => {
                    self.symbols.lucet_trap_manifest_len = Some(sym.clone())
                }
                "wasm_data_segments" => self.symbols.wasm_data_segments = Some(sym.clone()),
                "wasm_data_segments_len" => self.symbols.wasm_data_segments_len = Some(sym.clone()),
                "lucet_heap_spec" => self.symbols.lucet_heap_spec = Some(sym.clone()),
                "lucet_globals_spec" => self.symbols.lucet_globals_spec = Some(sym.clone()),
                "guest_sparse_page_data" => self.symbols.guest_sparse_page_data = Some(sym.clone()),
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

        self.heap_spec = self.parse_heap_spec();
        self.globals_spec = self.parse_globals_spec();
        self.data_segments = self.parse_data_segments();
        self.trap_manifest = self.parse_trap_manifest();
        self.sparse_page_data = self.parse_sparse_page_data();
    }

    fn parse_heap_spec(&self) -> Option<HeapSpec> {
        if let Some(ref sym) = self.symbols.lucet_heap_spec {
            let mut spec = HeapSpec {
                reserved_size: 0,
                guard_size: 0,
                initial_size: 0,
                max_size: None,
            };

            let serialized = self.read_memory(sym.st_value, sym.st_size).unwrap();
            let mut rdr = Cursor::new(serialized);
            spec.reserved_size = rdr.read_u64::<LittleEndian>().unwrap();
            spec.guard_size = rdr.read_u64::<LittleEndian>().unwrap();
            spec.initial_size = rdr.read_u64::<LittleEndian>().unwrap();

            let max_size = rdr.read_u64::<LittleEndian>().unwrap();
            let max_size_valid = rdr.read_u64::<LittleEndian>().unwrap();

            if max_size_valid == 1 {
                spec.max_size = Some(max_size);
            } else {
                spec.max_size = None;
            }

            Some(spec)
        } else {
            None
        }
    }

    fn parse_globals_spec(&self) -> Option<GlobalsSpec> {
        if let Some(ref sym) = self.symbols.lucet_globals_spec {
            let mut spec = GlobalsSpec { count: 0 };

            let serialized = self.read_memory(sym.st_value, sym.st_size).unwrap();
            let mut rdr = Cursor::new(serialized);
            spec.count = rdr.read_u64::<LittleEndian>().unwrap();

            Some(spec)
        } else {
            None
        }
    }

    fn parse_data_segments(&self) -> Option<DataSegments> {
        if let Some(ref data_sym) = self.symbols.wasm_data_segments {
            if let Some(ref data_len_sym) = self.symbols.wasm_data_segments_len {
                let mut data_segments = DataSegments {
                    segments: Vec::new(),
                };

                // TODO: validate that sym.st_size == 4
                let buffer = self
                    .read_memory(data_len_sym.st_value, data_len_sym.st_size)
                    .unwrap();
                let mut rdr = Cursor::new(buffer);
                let data_len = rdr.read_u32::<LittleEndian>().unwrap();
                // TODO: validate that data_len == data_sym.st_size

                let buffer = self
                    .read_memory(data_sym.st_value, data_sym.st_size)
                    .unwrap();
                let mut rdr = Cursor::new(&buffer);

                while rdr.position() < data_len as u64 {
                    let _memory_index = rdr.read_u32::<LittleEndian>();
                    // TODO: validate that memory_index == 0
                    let offset = rdr.read_u32::<LittleEndian>().unwrap();
                    let len = rdr.read_u32::<LittleEndian>().unwrap();

                    let pos = rdr.position() as usize;
                    let data_slice = &buffer[pos..pos + len as usize];

                    let mut data = Vec::new();
                    data.extend_from_slice(data_slice);

                    let pad = (8 - (pos + len as usize) % 8) % 8;
                    let new_pos = pos as u64 + len as u64 + pad as u64;
                    rdr.set_position(new_pos);

                    data_segments.segments.push(DataSegment {
                        offset: offset,
                        len: len,
                        data: data,
                    });
                }

                Some(data_segments)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_sparse_page_data(&self) -> Option<SparsePageData> {
        if let Some(ref sparse_sym) = self.symbols.guest_sparse_page_data {
            let mut sparse_page_data = SparsePageData { pages: Vec::new() };
            let buffer = self
                .read_memory(sparse_sym.st_value, sparse_sym.st_size)
                .unwrap();
            let buffer_len = buffer.len();
            let mut rdr = Cursor::new(buffer);
            let num_pages = rdr.read_u64::<LittleEndian>().unwrap();
            if buffer_len != size_of::<u64>() + num_pages as usize * size_of::<u64>() {
                eprintln!("size of sparse page data doesn't match the number of pages specified");
                None
            } else {
                for _ in 0..num_pages {
                    let ptr = rdr.read_u64::<LittleEndian>().unwrap() as *const u8;
                    sparse_page_data.pages.push(ptr);
                }
                Some(sparse_page_data)
            }
        } else {
            None
        }
    }

    fn parse_trap_manifest(&self) -> Option<TrapManifest> {
        let trap_manifest: elf::sym::Sym;
        let trap_manifest_len: elf::sym::Sym;

        // Make sure we have the necessary symbols first
        if let Some(ref tm) = self.symbols.lucet_trap_manifest {
            trap_manifest = tm.clone();
        } else {
            return None;
        }

        if let Some(ref tml) = self.symbols.lucet_trap_manifest_len {
            trap_manifest_len = tml.clone();
        } else {
            return None;
        }

        let mut manifest = TrapManifest {
            records: Vec::new(),
        };

        // Get the length of the manifest
        // TODO: return error if st_size != 4
        let serialized = self
            .read_memory(trap_manifest_len.st_value, trap_manifest_len.st_size)
            .unwrap();
        let mut rdr = Cursor::new(serialized);
        let trap_manifest_len = rdr.read_u32::<LittleEndian>().unwrap();

        // Find the manifest itself
        let serialized = self
            .read_memory(trap_manifest.st_value, trap_manifest.st_size)
            .unwrap();
        let mut rdr = Cursor::new(serialized);

        // Iterate through each row
        for _ in 0..trap_manifest_len {
            let func_start = rdr.read_u64::<LittleEndian>().unwrap();
            let func_len = rdr.read_u64::<LittleEndian>().unwrap();
            let traps = rdr.read_u64::<LittleEndian>().unwrap();
            let traps_len = rdr.read_u64::<LittleEndian>().unwrap();
            let func_name = self
                .get_func_name_for_addr(func_start)
                .unwrap_or("(not found)");

            let mut sites = Vec::new();

            // Find the table
            let serialized_table = self.read_memory(traps, 8 * traps_len).unwrap();
            let mut table_rdr = Cursor::new(serialized_table);

            // Iterate through each site
            for _ in 0..traps_len {
                let offset = table_rdr.read_u32::<LittleEndian>().unwrap();
                let trapcode = table_rdr.read_u32::<LittleEndian>().unwrap();

                sites.push(TrapSite { offset, trapcode });
            }

            manifest.records.push(TrapManifestRow {
                func_name: func_name.to_string(),
                func_addr: func_start,
                func_len: func_len,
                trap_count: traps_len,
                sites: sites,
            });
        }

        Some(manifest)
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

fn print_summary(summary: ArtifactSummary) {
    println!("Required Symbols:");
    println!(
        "  {:25}: {}",
        "lucet_trap_manifest",
        exists_to_str(&summary.symbols.lucet_trap_manifest)
    );
    println!(
        "  {:25}: {}",
        "lucet_trap_manifest_len",
        exists_to_str(&summary.symbols.lucet_trap_manifest_len)
    );
    println!(
        "  {:25}: {}",
        "wasm_data_segments",
        exists_to_str(&summary.symbols.wasm_data_segments)
    );
    println!(
        "  {:25}: {}",
        "wasm_data_segments_len",
        exists_to_str(&summary.symbols.wasm_data_segments_len)
    );
    println!(
        "  {:25}: {}",
        "guest_sparse_page_data",
        exists_to_str(&summary.symbols.guest_sparse_page_data)
    );
    println!(
        "  {:25}: {}",
        "lucet_heap_spec",
        exists_to_str(&summary.symbols.lucet_heap_spec)
    );
    println!(
        "  {:25}: {}",
        "lucet_globals_spec",
        exists_to_str(&summary.symbols.lucet_globals_spec)
    );

    println!("");
    println!("Exported Functions/Symbols:");
    for function_name in summary.exported_functions {
        println!("  {}", function_name);
    }

    println!("");
    println!("Imported Functions/Symbols:");
    for function_name in summary.imported_symbols {
        println!("  {}", function_name);
    }

    println!("");
    println!("Heap Specification:");
    if let Some(heap_spec) = summary.heap_spec {
        println!("  {:9}: {} bytes", "Reserved", heap_spec.reserved_size);
        println!("  {:9}: {} bytes", "Guard", heap_spec.guard_size);
        println!("  {:9}: {} bytes", "Initial", heap_spec.initial_size);
        if let Some(max_size) = heap_spec.max_size {
            println!("  {:9}: {} bytes", "Maximum", max_size);
        } else {
            println!("  {:9}: None", "Maximum");
        }
    } else {
        println!("  {}", "MISSING!".red().bold());
    }

    println!("");
    println!("Globals Specification:");
    if let Some(globals_spec) = summary.globals_spec {
        println!("  {:6}: {}", "Count", globals_spec.count);
    } else {
        println!("  {}", "MISSING!".red().bold());
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

    println!("");
    println!("Sparse page data:");
    if let Some(sparse_page_data) = summary.sparse_page_data {
        println!("  {:6}: {}", "Count", sparse_page_data.pages.len());
        let mut allempty = true;
        for (i, page) in sparse_page_data.pages.iter().enumerate() {
            if !page.is_null() {
                allempty = false;
                println!("  Page[{}]: {:p}", i, *page);
            }
        }
        if allempty {
            println!("  (all pages empty)");
        } else {
            println!("  (empty pages omitted)");
        }
    } else {
        println!("  {}", "MISSING!".red().bold());
    }

    println!("");
    println!("Trap Manifest:");
    if let Some(trap_manifest) = summary.trap_manifest {
        for row in trap_manifest.records {
            println!("  {:25} {} traps", row.func_name, row.trap_count);
            println!("      {:?}", row.sites);
        }
    } else {
        println!("  {}", "MISSING!".red().bold());
    }
}

fn exists_to_str<T>(p: &Option<T>) -> colored::ColoredString {
    return match p {
        Some(_) => "exists".green(),
        None => "MISSING!".red().bold(),
    };
}
