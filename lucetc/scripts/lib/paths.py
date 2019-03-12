# python syntax is a reasonable key-value config file, so installers should
# create variables 'clang_default_bin', 'wasmld_default_bin' etc at the
# beginning of this file to override the defaults set here.

import os

lucetc_src_directory = os.path.dirname(os.path.dirname(os.path.dirname(os.path.realpath(__file__))))
libc_path = os.path.join(os.path.dirname(lucetc_src_directory), 'lucet-libc', 'build')

clang_bin = os.environ.get("LUCET_CLANG", locals().get('clang_default_bin', 'clang'))
wasmld_bin = os.environ.get("LUCET_WASM_LD", locals().get('wasmld_default_bin', 'wasm-ld'))
lucetc_bin = os.environ.get("LUCETC_BIN_PATH", locals().get('lucetc_default_bin',
    os.path.join(os.path.dirname(lucetc_src_directory), 'target', 'debug', 'lucetc')))
libc_sysroot_path = os.environ.get("LUCET_LIBC_SYSROOT_PATH", locals().get('libc_sysroot_default_path', os.path.join(libc_path, 'sysroot')))
libc_lib_path = os.environ.get("LUCET_LIBC_LIB_PATH", locals().get('libc_lib_default_path', os.path.join(libc_path, 'wasmlib')))
