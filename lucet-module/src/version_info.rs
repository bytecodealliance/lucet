use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::cmp::min;
use std::fmt;
use std::io;

/// VersionInfo is information about a Lucet module to allow the Lucet runtime to determine if or
/// how the module can be loaded, if so requested. The information here describes implementation
/// details in runtime support for `lucetc`-produced modules, and nothing higher level.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    major: u16,
    minor: u16,
    patch: u16,
    reserved: u16,
    /// `version_hash` is either all nulls or the first eight ascii characters of the git commit
    /// hash of wherever this Version is coming from. In the case of a compiled lucet module, this
    /// hash will come from the git commit that the lucetc producing it came from. In a runtime
    /// context, it will be the git commit of lucet-runtime built into the embedder.
    ///
    /// The version hash will typically populated only in release builds, but may blank even in
    /// that case: if building from a packaged crate, or in a build environment that does not have
    /// "git" installed, `lucetc` and `lucet-runtime` will fall back to an empty hash.
    version_hash: [u8; 8],
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if u64::from_ne_bytes(self.version_hash) != 0 {
            write!(
                fmt,
                "-{}",
                std::str::from_utf8(&self.version_hash).unwrap_or("INVALID")
            )?;
        }
        Ok(())
    }
}

impl VersionInfo {
    pub fn new(major: u16, minor: u16, patch: u16, version_hash: [u8; 8]) -> VersionInfo {
        VersionInfo {
            major,
            minor,
            patch,
            reserved: 0x8000,
            version_hash,
        }
    }

    /// A more permissive version check than for version equality. This check will allow an `other`
    /// version that is more specific than `self`, but matches for data that is available.
    pub fn compatible_with(&self, other: &VersionInfo) -> bool {
        if !(self.valid() || other.valid()) {
            return false;
        }

        if self.major == other.major && self.minor == other.minor && self.patch == other.patch {
            if self.version_hash == [0u8; 8] {
                // we aren't bound to a specific git commit, so anything goes.
                true
            } else {
                self.version_hash == other.version_hash
            }
        } else {
            false
        }
    }

    pub fn write_to<W: WriteBytesExt>(&self, w: &mut W) -> io::Result<()> {
        w.write_u16::<LittleEndian>(self.major)?;
        w.write_u16::<LittleEndian>(self.minor)?;
        w.write_u16::<LittleEndian>(self.patch)?;
        w.write_u16::<LittleEndian>(self.reserved)?;
        w.write(&self.version_hash).and_then(|written| {
            if written != self.version_hash.len() {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "unable to write full version hash",
                ))
            } else {
                Ok(())
            }
        })
    }

    pub fn read_from<R: ReadBytesExt>(r: &mut R) -> io::Result<Self> {
        let mut version_hash = [0u8; 8];
        Ok(VersionInfo {
            major: r.read_u16::<LittleEndian>()?,
            minor: r.read_u16::<LittleEndian>()?,
            patch: r.read_u16::<LittleEndian>()?,
            reserved: r.read_u16::<LittleEndian>()?,
            version_hash: {
                r.read_exact(&mut version_hash)?;
                version_hash
            },
        })
    }

    pub fn valid(&self) -> bool {
        self.reserved == 0x8000
    }

    pub fn current(current_hash: &'static [u8]) -> Self {
        let mut version_hash = [0u8; 8];

        for i in 0..min(version_hash.len(), current_hash.len()) {
            version_hash[i] = current_hash[i];
        }

        // The reasoning for this is as follows:
        // `SerializedModule`, in version before version information was introduced, began with a
        // pointer - `module_data_ptr`. This pointer would be relocated to somewhere in user space
        // for the embedder of `lucet-runtime`. On x86_64, hopefully, that's userland code in some
        // OS, meaning the pointer will be a pointer to user memory, and will be below
        // 0x8000_0000_0000_0000. By setting `reserved` to `0x8000`, we set what would be the
        // highest bit in `module_data_ptr` in an old `lucet-runtime` and guarantee a segmentation
        // fault when loading these newer modules with version information.
        VersionInfo::new(
            env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
            version_hash,
        )
    }
}
