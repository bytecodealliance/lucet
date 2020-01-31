# Compiling Lucet from scratch

Specific instructions are available for [some flavors of Linux](./Compiling-on-Linux.md) and for
[macOS](./Compiling-on-macOS.md) (experimental).

If you are using another platform, or if the provided instructions are not working, it may be
helpful to try adapting the setup code in the `Dockerfile` that defines the Lucet continuous
integration environment. While the image is defined in terms of Ubuntu, many of the packages are
available through other package managers and operating systems.

```Dockerfile
{{#include ../../Dockerfile}}
```
