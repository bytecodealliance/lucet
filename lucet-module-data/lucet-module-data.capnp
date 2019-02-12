@0xe529fa6099d786bc;

struct ModuleData {
  heapSpec @0 :HeapSpec;
  sparseData @1 :SparseData;
  globalsSpec @2 :List(GlobalSpec);
}

struct HeapSpec {
  reservedSize @0 :UInt64;
  guardSize @1 :UInt64;
  initialSize @2 :UInt64;
  maxSize :union {
     maxSize @3 :UInt64;
     none @4 :Void;
  }
}

struct SparseData {
  chunks @0 :List(SparseChunk);
}

struct SparseChunk {
  contents :union {
    empty @0 :Void;
    full @1 :Data; # will be exactly 4k
  }
}


struct GlobalSpec {
  global :union {
    def @0 :GlobalDef;
    import @1 :GlobalImport;
  }
  export :union {
    name @2 :Text;
    none @3 :Void;
  }
}

struct GlobalDef {
  initVal @0 :UInt64;
}

struct GlobalImport {
   module @0 :Text;
   field @1 :Text;
}
