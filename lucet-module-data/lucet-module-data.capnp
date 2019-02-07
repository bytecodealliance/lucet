@0xe529fa6099d786bc;

struct ModuleData {
  heapSpec @0 :HeapSpec;
  sparseData @1 :SparseData;
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
  chunks @0: List(SparseChunk);
}

struct SparseChunk {
  contents :union {
    empty @0: Void;
    full @1: Data; # will be exactly 4k
  }
}
