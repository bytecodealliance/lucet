# An AssemblyScript layer for the WebAssembly System Interface (WASI)

[WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md) is an API providing access to the external world to WebAssembly modules.

WASA is an effort to expose the WASI standard set of system calls to AssemblyScript.

## Usage

Example usage of the `Console` and `Environ` classes:

```typescript
import "allocator/arena";
import { Console, Environ } from "../node_modules/wasa/assembly";

let env = new Environ();
let home = env.get("HOME") as String;
Console.log(home);
```