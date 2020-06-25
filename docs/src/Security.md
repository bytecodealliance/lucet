# Lucet security overview

This document provides a high-level summary of the security architecture of the Lucet project. It is meant to be used for orientation and a starting point for deploying a secure embedding of Lucet.

## Security model

The Lucet project aims to provide support for secure execution of untrusted code. The project does not provide a complete secure sandbox framework at this time; security is achieved through a combination of Lucet-supplied security controls and user-supplied security controls.

At a high level, this jointly-constructed security architecture aims to prevent untrusted input, data, and activity from compromising the security of trusted components. It also aims to prevent an untrusted actor from compromising the security (e.g. data and activity) of another untrusted actor. For example, one user of a Lucet embedding should not be able to affect the security of another user of the same Lucet embedding.

Some security requirements for the Lucet project have not been implemented yet. See the remainder of this document as well as [project Github issues](https://github.com/bytecodealliance/lucet/issues) for more information. Note that even when Lucet project security goals have been met, overall system security requirements will vary by embedding.

The Lucet security model can be summarized via two simplified execution scenarios: compiling/loading of sandboxed guest code and execution of untrusted guest programs. These scenarios are described in terms of the following levels.

- Trusted: refers to code, processes, or inputs that are fully trusted and generally controlled by the administrator of a system that runs or embeds Lucet components.
- Untrusted: refers to code, processes, or inputs that are completely untrusted and generally supplied by a third party. For example, user-supplied Wasm code is untrusted.

The scenarios are modeled as simplified data flow diagrams below. [draw.io](https://draw.io) diagram source files are available [here](assets/lucet_dfds.xml).

### Compile/load scenario

![](assets/security_dfd_cl.png)

In the compile/load scenario, a user provides untrusted WebAssembly code to the [`lucetc`](https://github.com/bytecodealliance/lucet/tree/main/lucetc) compiler. The `lucetc` compiler consumes this code along with trusted bindings and produces a shared object file. A trusted application (e.g. server) that embeds `lucet-runtime` then loads the guest program.

### Program execution scenario

![](assets/security_dfd_pe.png)

In the program execution scenario, an untrusted third party end-user sends data to a trusted server that has loaded a guest program (via the compile/load scenario above). The trusted server handles this data and passes it to an instance of the untrusted guest program for processing. The guest program may call into trusted server APIs to perform privileged processing, such as further communication with the end-user, untrusted network endpoints, etc. before execution terminates.

## Security requirements

This section summarizes salient security requirements for the Lucet projects in terms of high-level attack scenarios. As mentioned above, Lucet does not provide a complete secure sandbox framework at this time; security is achieved through a combination of Lucet-supplied security controls and user-supplied security controls.

### Attacks against compilation process

An attacker may be able to supply a malicious input file to the `lucetc` compiler toolchain in the context of the “compile/load” scenario above, with a goal of compromising `lucetc` and/or the host system it is executing within.

Lucet is designed to prevent elevation of privilege attacks and against the `lucetc` compiler toolchain. Due to the nature of WebAssembly application, upstream components of the `lucetc` compiler (particularly [Cranelift](https://github.com/bytecodealliance/cranelift)) generally have a similar design goals in this respect, and have corresponding security measures in place. The Lucet project has undergone an initial security assessment.

Bugs in `lucetc` that can lead to information leaks, elevation of privilege (e.g. arbitrary remote code execution) and otherwise compromise security attributes are considered security vulnerabilities in the context of the Lucet project.

Attack vectors stemming from asymmetric consumption of resources inherent in compilation processes, for example consumption of CPU or memory for large or complex inputs, should be addressed by user/administrator via environmental controls or similar. For example, a `lucetc` deployment could limit input size earlier in the processing flow, include cgroup runtime controls, etc.

Note that an evolving compiler toolchain like `lucetc` presents a rich attack surface that will likely require ongoing patching of vulnerabilities. It is highly recommended that additional protections common classes of attacks be deployed by administrators for defense-in-depth. For example, the [terrarium project](https://wasm.fastlylabs.com/) runs `lucetc` compilation jobs in minimal, single-use, security-hardened containers in an isolated environment subject to runtime security monitoring.

### Guest-to-host attacks

An attacker can supply malicious guest code to a Lucet embedding. Bugs in `lucetc`, `lucet-runtime`, or any other project components that allow code generated by an attacker to elevate privileges against the embedding host, crash the host, leak host data, or otherwise compromise the host’s security are considered security vulnerabilities. Correspondingly, bugs in Lucet that compromise of security policies of system components (e.g. [WASI capabilities policies](https://github.com/bytecodealliance/wasmtime/blob/main/docs/WASI-overview.md)) are considered security vulnerabilities.

Lucet leverages WebAssembly semantics, control flow, and operational memory isolation models to prevent broad classes of attacks against the host embedding (see the [WebAssembly docs](https://webassembly.org/docs/security/) for details). Specifically, Lucet provides WebAssembly-based mechanisms for isolating most faults to a specific instance of guest program; in these cases mitigations can be applied (e.g. alerting, guest banning, etc.) and execution of the host process can continue unabated. Lucet is compatible with the [WebAssembly System Interface (WASI)](https://wasi.dev) API for system interfaces, which supplies a capabilities-based security model for system resource access. Lucet is designed to provide a baseline for integration with additional host sandboxing technologies, such as seccomp-bpf.

Host function call bindings supplied by the Lucet user/administrator are analogous to WebAssembly imported functions. Lucet project components aim to generate code that provides ABI-level consistency checking of function call arguments (work in progress), but vulnerabilities explicitly defined in host-side functionality supplied by Lucet administrators (e.g. memory corruption in an embedding server’s C code) is considered out-of-scope for the Lucet project.

#### Caveats

- Lucet does not provide complete protection against transient/speculative execution attacks against the host. Efforts are underway in `lucetc` and upstream projects to supply industry-standard protections to generated native code, but Lucet users/administrators must deploy additional defenses, such as protecting imported function APIs from speculative execution, applying privilege separation, [site isolation](https://www.chromium.org/Home/chromium-security/site-isolation), [sandboxing technology](https://wiki.mozilla.org/Security/Sandbox/Seccomp#Intro_to_seccomp_and_seccomp-bpf) and so on.
- Support for automated ABI-level consistency checking of function call arguments is not complete. In the meantime, Lucet users/administrators must implement this checking.
- Lucet is a new technology and under active development. Designers and architects should plan to monitor releases and regularly patch Lucet to benefit from remediation of security vulnerabilities.

### Guest-to-guest attacks

This scenario is similar to the previous one, except an attacker is targeting another guest. Similarly, bugs in `lucetc`, `lucet-runtime`, or any other project components that allow code generated by an attacker to leak data of other guest or other compromise the security of other guests are considered vulnerabilities.

The protections, responsibilities, and caveats defined in the previous section apply to this attack scenario as well.

### Attacks against guest programs

An attacker may attempt to exploit a victim guest program that is executing in a Lucet host embedding. Lucet provides WebAssembly-based security guarantees for guest programs, but WebAssembly programs may still be vulnerable to exploitation. For example, memory allocated within a linear memory region [may not have conventional protections in place](https://00f.net/2018/11/25/webassembly-doesnt-make-unsafe-languages-safe/), [type confusion](https://www.fastly.com/blog/hijacking-control-flow-webassembly) or [other basic memory corruption vulnerabilities](https://i.blackhat.com/us-18/Thu-August-9/us-18-Lukasiewicz-WebAssembly-A-New-World-of-Native_Exploits-On-The-Web-wp.pdf) that are not obviated by WebAssembly may be present in guest programs, and so on. It is the Lucet administrator’s responsibility to protect vulnerable guest program logic beyond WebAssembly-provided safety measures.

## Report a security issue

See the project's [SECURITY.md](../../SECURITY.md)
