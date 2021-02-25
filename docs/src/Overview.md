{{#include ../../README.md}}

## Getting started

To learn how to set up the toolchain, and then compile and run your first WebAssembly application
using Lucet, see [Using Lucet](./Using-lucet.md).

## Development environment

Lucet is developed and tested on x86-64 Linux, with experimental support for macOS. For compilation
instructions, see [Compiling](./Compiling.md).

## Supported languages and platforms

Lucet supports running WebAssembly programs written in C and C++ (via `clang`), Rust, and
AssemblyScript. It does not yet support the entire WebAssembly spec, but full support is
[planned](./lucet-spectest.md).

Lucet's runtime currently supports x86-64 based Linux systems, with experimental support for macOS.

## Security

The Lucet project aims to provide support for secure execution of untrusted code. Security is
achieved through a combination of Lucet-supplied security controls and user-supplied security
controls. See [Security](./Security.md) for more information on the Lucet security model.

### Reporting Security Issues

The Lucet project team welcomes security reports and is committed to providing prompt attention to
security issues. Security issues should be reported privately via [Fastly’s security issue reporting
process](https://www.fastly.com/security/report-security-issue). Remediation of security
vulnerabilities is prioritized. The project teams endeavors to coordinate remediation with
third-party stakeholders, and is committed to transparency in the disclosure process.
