import os
import tempfile
import atexit
import subprocess
import sys

import paths

# Tempfile utilities

tmpfiles = list()
def get_tmpfile():
    fd, filename = tempfile.mkstemp('.o')
    os.close(fd)
    tmpfiles.append(filename)
    return filename

def delete_tmpfiles():
    for filename in tmpfiles:
        if os.path.exists(filename):
            os.remove(filename)

atexit.register(delete_tmpfiles)

def warning(string):
    sys.stderr.write("Warning: %s\n" % string)

# Compilation step functions

def version_works(path):
    try:
        with open(os.devnull, 'w') as FNULL:
            ret = subprocess.call([path, "--version"], stdout=FNULL)
            return ret == 0
    except Exception as e:
        return False


def c_to_wasm_obj(in_filename, out_filename, args, unknown_args):
    if args.verbose:
        print "Going to compile C file to WASM obj: %s -> %s" % (in_filename, out_filename)

    if not version_works(paths.clang_bin):
        raise Exception("no clang executable found for '%s'" % (paths.clang_bin))

    if not os.path.exists(paths.libc_sysroot_path):
        raise Exception("libc not found at expected path %s" % (paths.libc_sysroot_path))

    cmd = [paths.clang_bin,
           '-target',
           'wasm32-wasm',
           '-fvisibility=default',
           '--sysroot=%s' % paths.libc_sysroot_path]

    cmd.extend(unknown_args)
    cmd.extend(['-o', out_filename, '-c', in_filename])

    if args.verbose:
        print ' '.join(cmd)
    try:
        ret = subprocess.call(cmd)
    except Exception as e:
        raise Exception("error calling clang: %s" % e)
    if ret != 0:
        raise ValueError("clang returned %d" % ret)

def link_wasm_objs(in_filenames, out_filename, args, unknown_args):
    if args.verbose:
        print "Going to link WASM objects to a single WASM object: %r -> %s" % (in_filenames, out_filename)

    # TODO: support passing linker args (but we need to filter them)
    #linker_args = list()
    #for arg in unknown_args:
    #    if arg.startswith('-Wl,'):
    #        fields = arg.split(',')
    if not version_works(paths.wasmld_bin):
        raise Exception("no wasm-ld executable found for '%s'" % (paths.wasmld_bin))

    if not os.path.exists(os.path.join(paths.libc_lib_path, 'libc.a')):
        raise Exception("libc.a not found in search path %s" % (paths.libc_lib_path))

    cmd = [paths.wasmld_bin,
           '--allow-undefined',
           '--no-entry',
           '--no-threads',
           '-L%s' % paths.libc_lib_path,
           '-lc']

    cmd.extend(['-o', out_filename])
    cmd.extend(in_filenames)

    if args.verbose:
        print ' '.join(cmd)

    ret = subprocess.call(cmd)
    if ret != 0:
        raise ValueError("wasm-ld returned %d" % ret)

def wasm_obj_to_so(in_filename, out_filename, args, unknown_args):
    if args.verbose:
        print "Going to compile a WASM obj to a Native SO: %s -> %s" % (in_filename, out_filename)

    if not os.path.exists(paths.lucetc_bin):
        raise Exception("lucetc not found at expected path %s" % (paths.lucetc_bin))

    cmd = [paths.lucetc_bin,
           in_filename,
           '-o',
           out_filename]

    if args.bindings:
        for binding in args.bindings:
            cmd += [ '--bindings', binding ]

    if args.verbose:
        print ' '.join(cmd)

    ret = subprocess.call(cmd)
    if ret != 0:
        raise ValueError("lucetc returned %d" % ret)

# Workflow generation

def filename_input_type(filename):
    basename, suffix = os.path.splitext(filename)
    if suffix == '.c':
        return 'c'
    elif suffix == '.o' or suffix == '.wasm':
        return 'wasm_obj'
    elif suffix == '.s' or suffix == '.S':
        return 'wat'
    elif suffix == '.a':
        return 'wasm_ar'
    else:
        return None

def input_types(args):
    ins = list()
    for filename in args.input_files:
        if hasattr(args, 'input_language') and args.input_language is not None:
            # If input lang is specified directly, use that
            ins.append((filename, args.input_language))
        else:
            # Otherwise, determine input lang by filename
            ins.append((filename, filename_input_type(filename)))
    return ins

def filename_output_type(filename):
    basename, suffix = os.path.splitext(filename)
    if suffix == '.o' or suffix == '.wasm':
        return 'wasm_obj'
    elif suffix == '.wat':
        return 'wat'
    elif suffix == '.clif':
        return 'clif'
    elif suffix == '.ar':
        return 'wasm_ar'
    elif suffix == '.so':
        return 'so'
    else:
        return None

def arg_output_type(args):
    if args.emit_wasm:
        return 'wasm_obj'
    elif args.emit_wat:
        return 'wat'
    elif args.emit_clif:
        return 'clif'
    elif args.emit_ar:
        return 'wasm_ar'
    elif args.emit_obj:
        return 'obj'
    elif args.emit_so:
        return 'so'
    else:
        return None

def output_type(args):
    if args.output_file is None:
        args.output_file = "a.out"

    # Infer a type from the output filename
    filename_type = filename_output_type(args.output_file)
    # First try the most explicit options
    output_type = arg_output_type(args)
    if filename_type is not None and output_type is not None and filename_type != output_type:
        warning("output type set to %s; output filename has type %s" % (output_type, filename_type))

    # Then try the GCC-standard options
    if output_type is None:
        if hasattr(args, 'c') and args.c:
            output_type = 'wasm_obj'
            if filename_type is not None and filename_type != output_type:
                warning("inferred output type wasm_obj from -c flag; output filename has type %s" % filename_type)
        elif hasattr(args, 'S') and args.S:
            output_type = 'wat'
            if filename_type is not None and filename_type != output_type:
                warning("inferred output type wat from -S flag; output filename has type %s" % filename_type)

    # If all else fails, determine the output by the filename
    if output_type is None:
        if filename_type is None:
            raise Exception("could not infer output type")
        output_type = filename_type

    return (args.output_file, output_type)

def compiler_workflow(args):
    ins = input_types(args)
    out_filename, out_type = output_type(args)

    if out_type == 'wasm_obj':
        assert len(ins) == 1
        in_filename, in_type = ins[0]
        if in_type == 'c':
            return [(c_to_wasm_obj, in_filename, out_filename)]
        elif in_type == 'wat':
            return [(wat_to_wasm_obj, in_filename, out_filename)]
        else:
            raise Exception('wasm objects can only be created from C or wat file')

    elif out_type == 'so':
        return link_so(ins, out_filename)
    else:
        raise Exception('invalid output type %s for %s' % (out_type, out_filename))


def linker_workflow(args):
    ins = input_types(args)
    out_filename, out_type = output_type(args)

    if out_type == 'so':
        return link_so(ins, out_filename)
    elif out_type == 'wasm_obj':
        return [(link_wasm_objs, [f for (f,t) in ins], out_filename)]
    else:
        raise Exception('invalid output type %s for %s' % (out_type, out_filename))

def link_so(ins, out_filename) :
    assert len(ins) > 0
    if not all(t == 'wasm_ar' or t == 'wasm_obj' for (f,t) in ins):
        raise Exception('shared objects can only be created from wasm objects or libraries')

    tmp_filename = get_tmpfile()
    wasm_obj_filenames = list(f for (f,t) in ins)
    return [(link_wasm_objs, wasm_obj_filenames, tmp_filename),
            (wasm_obj_to_so, tmp_filename, out_filename)]
