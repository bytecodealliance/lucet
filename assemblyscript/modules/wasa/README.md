# An AssemblyScript layer for the WebAssembly System Interface (WASI)

[WASI](https://wasi.dev) is an API providing access to the external world to WebAssembly modules.

WASA is an effort to expose the WASI standard set of system calls to AssemblyScript.

## Usage

Example usage of the `Console` and `Environ` classes:

```typescript
import { Console, Environ } from "../node_modules/wasa/assembly";

let env = new Environ();
let home = env.get("HOME")!;
Console.log(home);
```
