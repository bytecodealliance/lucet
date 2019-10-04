# lucet-validate

Validates a WebAssembly module against a witx spec.

## What is witx?

* Witx is a specification languaged developed as part of the
  [WASI](https://github.com/WebAssembly/WASI) effort. The `witx` crate lives in
  that repository, as well as `.witx` files that describe the WASI standard.

* A Witx specification is parsed and validated from `.witx` files by the `witx`
  crate. The set of types and modules defined by these files is called a Witx document.

* A Witx specification contains modules, and modules contain interface functions.
  These are functions defined in terms of parameters (inputs) and results
  (outputs), all of which can have complex types like pointers, arrays, strings,
  structs etc.

* Each interface function has a method to calculate its type signature in terms
  of the "core" WebAssembly types (i32, i64, f32, f64 used in WebAssembly 1.0
  function types). This calculation takes into account that some complex types
  are passed as pointers into linear memory, or a pointer-length pair, while
  others (smaller ints like u8 or s16) can be represented by atomic values
  (i32, in this example).


## What is validated?

* The WebAssembly module itself is validated to be WebAssembly 1.0. We don't
  support validating extensions to the spec yet but ought to be able to without
  any issues.

* Each import of the WebAssembly module is validated to be present, and have
  the expected core type signature, given by the Witx document.

* If the Validator is set to validate an `wasi-exe`, it additionally checks
  that the module exports a function named `_start` with the type signature `[]
  -> ()`. (This is not to be confused with having a `start` section, which is a
  different concept from the WASI executable entrypoint `_start`.)


## What is not?

* The WebAssembly module does not contain enough information to determine that
  the core types found in the type signature are used by the WebAssembly
  program in a way that matches the complex types (strings arrays structs etc)
  in the witx document. This property could only be validated in the source
  language before it is compiled to WebAssembly.


