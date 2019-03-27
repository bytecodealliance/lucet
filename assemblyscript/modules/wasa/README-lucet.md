# Notes on WASA and Lucet integration

The WASA module contains bindings for the WASI system calls currently
implemented in Lucet.

The `bindings.json` file contains the bindings import table for `lucetc`.

Example usage:

```sh
npm run asbuild:optimized

lucetc build/optimized.wasm -o app \
  --bindings /opt/lucet/share/assemblyscript/modules/wasa/bindings.json

lucet-wasi ./app
```
