use super::*;

pub fn generate(
    pretty_writer: &mut PrettyWriter,
    target: Target,
    backend_config: BackendConfig,
) -> Result<(), IDLError> {
    let prelude = r"
#include <assert.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>";

    for line in prelude.lines() {
        pretty_writer.write_line(line.as_ref())?;
    }
    pretty_writer.eob()?;

    if backend_config.zero_native_pointers {
        pretty_writer.write_line(
            r"#define ZERO_NATIVE_POINTERS // Avoid serializing native pointers".as_ref(),
        )?;
    } else if !(target.is_reference_alignment_compatible()
        && target.uses_reference_target_endianness())
    {
        pretty_writer.write_line(
            r"// #define ZERO_NATIVE_POINTERS // Define to avoid serializing native pointers"
                .as_ref(),
        )?;
    }

    let prelude = r"
#ifndef ___REFERENCE_COMPATIBLE_ALIGNMENT
# if defined(__amd64) || defined(__amd64__) || defined(__x86_64__) || defined(_M_X64) || defined(_M_AMD64) || \
     defined(__EMSCRIPTEN__)
#  define ___REFERENCE_COMPATIBLE_ALIGNMENT
# endif
#endif

#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
# define ___le_uint16_t(X) (X)
# define ___le_uint32_t(X) (X)
# define ___le_uint64_t(X) (X)
# define ___le_int16_t(X)  (X)
# define ___le_int32_t(X)  (X)
# define ___le_int64_t(X)  (X)
# define ___le_float(X)    (X)
# define ___le_double(X)   (X)
#else
# define ___bswap16(X) __builtin_bswap16(X)
# define ___bswap32(X) __builtin_bswap32(X)
# define ___bswap64(X) __builtin_bswap64(X)

# define ___le_uint16_t(X) ___bswap16(X)
# define ___le_uint32_t(X) ___bswap32(X)
# define ___le_uint64_t(X) ___bswap64(X)
# define ___le_int16_t(X)  ((int16_t) ___bswap16((uint16_t) (X)))
# define ___le_int32_t(X)  ((int32_t) ___bswap32((uint32_t) (X)))
# define ___le_int64_t(X)  ((int64_t) ___bswap64((uint64_t) (X)))

static inline float ___le_float(float X) {
    uint32_t X_; float Xf; memcpy(&X_, &X, sizeof X_);
    X_ = ___le_uint32_t(X_); memcpy(&Xf, &X_, sizeof Xf);
    return Xf;
}

static inline double ___le_double(double X) {
    uint64_t X_; double Xd; memcpy(&X_, &X, sizeof X_);
    X_ = ___le_uint64_t(X_); memcpy(&Xd, &X_, sizeof Xd);
    return Xd;
}
#endif

#if !defined(ZERO_NATIVE_POINTERS) && defined(___REFERENCE_COMPATIBLE_ALIGNMENT) && \
    __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
#  define ___REFERENCE_COMPATIBLE_ENCODING
#endif

#define BYTES_PTR 8

#if UINTPTR_MAX > 0xFFFFFFFFULL
# define ___POINTER_PAD(O)
#elif UINTPTR_MAX > 0xFFFFUL
# define ___POINTER_PAD(O) uint8_t ___ptrpad_ ## O ## _[4];
#else
# define ___POINTER_PAD(O) uint8_t ___ptrpad_ ## O ## _[6];
#endif
";
    for line in prelude.lines() {
        pretty_writer.write_line(line.as_ref())?;
    }
    pretty_writer.eob()?;
    Ok(())
}
