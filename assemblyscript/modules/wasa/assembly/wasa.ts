import {
  advice,
  args_get,
  args_sizes_get,
  clock_res_get,
  clock_time_get,
  clockid,
  dircookie,
  environ_get,
  environ_sizes_get,
  errno,
  fd_advise,
  fd_allocate,
  fd_close,
  fd_datasync,
  fd_fdstat_get,
  fd_fdstat_set_flags,
  fd_filestat_get,
  fd_filestat_set_size,
  fd_filestat_set_times,
  fd_prestat_dir_name,
  fd_read,
  fd_readdir,
  fd_seek,
  fd_sync,
  fd_tell,
  fd_write,
  fd,
  fdflags,
  fdstat,
  whence,
  filesize,
  filestat,
  filetype,
  fstflags,
  lookupflags,
  oflags,
  path_create_directory,
  path_filestat_get,
  path_link,
  path_open,
  path_rename,
  path_remove_directory,
  path_symlink,
  path_unlink_file,
  proc_exit,
  random_get,
  rights,
} from "bindings/wasi";

/**
 * A WASA error
 */
export class WASAError extends Error {
  constructor(message: string = "") {
    super(message);
    this.name = "WASAError";
  }
}

/**
 * Portable information about a file
 */
export class FileStat {
  file_type: filetype;
  file_size: filesize;
  access_time: f64;
  modification_time: f64;
  creation_time: f64;

  constructor(st_buf: usize) {
    this.file_type = load<u8>(st_buf + 16);
    this.file_size = load<u64>(st_buf + 24);
    this.access_time = (load<u64>(st_buf + 32) as f64) / 1e9;
    this.modification_time = (load<u64>(st_buf + 40) as f64) / 1e9;
    this.creation_time = (load<u64>(st_buf + 48) as f64) / 1e9;
  }
}

/**
 * A descriptor, that doesn't necessarily have to represent a file
 */
export class Descriptor {
  /**
   * An invalid file descriptor, that can represent an error
   */
  static Invalid(): Descriptor { return new Descriptor(-1); };

  /**
   * The standard input
   */
  static Stdin(): Descriptor { return new Descriptor(0); };

  /**
   * The standard output
   */
  static Stdout(): Descriptor { return new Descriptor(1); };

  /**
   * The standard error
   */
  static Stderr(): Descriptor { return new Descriptor(2); };

  /**
   * Build a new descriptor from a raw WASI file descriptor
   * @param rawfd a raw file descriptor
   */
  constructor(readonly rawfd: fd) { }

  /**
   * Hint at how the data accessible via the descriptor will be used
   * @offset offset
   * @len length
   * @advice `advice.{NORMAL, SEQUENTIAL, RANDOM, WILLNEED, DONTNEED, NOREUSE}`
   * @returns `true` on success, `false` on error
   */
  advise(offset: u64, len: u64, advice: advice): bool {
    return fd_advise(this.rawfd, offset, len, advice) === errno.SUCCESS;
  }

  /**
   * Preallocate data
   * @param offset where to start preallocating data in the file
   * @param len bytes to preallocate
   * @returns `true` on success, `false` on error
   */
  allocate(offset: u64, len: u64): bool {
    return fd_allocate(this.rawfd, offset, len) === errno.SUCCESS;
  }

  /**
   * Wait for the data to be written
   * @returns `true` on success, `false` on error
   */
  fdatasync(): bool {
    return fd_datasync(this.rawfd) === errno.SUCCESS;
  }

  /**
   * Wait for the data and metadata to be written
   * @returns `true` on success, `false` on error
   */
  fsync(): bool {
    return fd_sync(this.rawfd) === errno.SUCCESS;
  }

  /**
   * Return the file type
   */
  fileType(): filetype {
    let st_buf = changetype<usize>(new ArrayBuffer(24));
    if (fd_fdstat_get(this.rawfd, changetype<fdstat>(st_buf)) !== errno.SUCCESS) {
      throw new WASAError("Unable to get the file type");
    }
    let file_type: u8 = load<u8>(st_buf);

    return file_type;
  }

  /**
   * Set WASI flags for that descriptor
   * @params flags: one or more of `fdflags.{APPEND, DSYNC, NONBLOCK, RSYNC, SYNC}`
   * @returns `true` on success, `false` on error
   */
  setFlags(flags: fdflags): bool {
    return fd_fdstat_set_flags(this.rawfd, flags) === errno.SUCCESS;
  }

  /**
   * Retrieve information about a descriptor
   * @returns a `FileStat` object`
   */
  stat(): FileStat {
    let st_buf = changetype<usize>(new ArrayBuffer(56));
    if (fd_filestat_get(this.rawfd, changetype<filestat>(st_buf)) !== errno.SUCCESS) {
      throw new WASAError("Unable to get the file information");
    }
    return new FileStat(st_buf);
  }

  /**
   * Change the size of a file
   * @param size new size
   * @returns `true` on success, `false` on error
   */
  ftruncate(size: u64 = 0): bool {
    return fd_filestat_set_size(this.rawfd, size) === errno.SUCCESS;
  }

  /**
   * Update the access time
   * @ts timestamp in seconds
   * @returns `true` on success, `false` on error
   */
  fatime(ts: f64): bool {
    return (
      fd_filestat_set_times(this.rawfd, (ts * 1e9) as u64, 0, fstflags.SET_ATIM) ===
      errno.SUCCESS
    );
  }

  /**
   * Update the modification time
   * @ts timestamp in seconds
   * @returns `true` on success, `false` on error
   */
  fmtime(ts: f64): bool {
    return (
      fd_filestat_set_times(this.rawfd, 0, (ts * 1e9) as u64, fstflags.SET_MTIM) ===
      errno.SUCCESS
    );
  }

  /**
   * Update both the access and the modification times
   * @atime timestamp in seconds
   * @mtime timestamp in seconds
   * @returns `true` on success, `false` on error
   */
  futimes(atime: f64, mtime: f64): bool {
    return (
      fd_filestat_set_times(this.rawfd, (atime * 1e9) as u64, (mtime * 1e9) as u64,
        fstflags.SET_ATIM | fstflags.SET_ATIM) === errno.SUCCESS
    );
  }

  /**
   * Update the timestamp of the object represented by the descriptor
   * @returns `true` on success, `false` on error
   */
  touch(): bool {
    return (
      fd_filestat_set_times(
        this.rawfd,
        0,
        0,
        fstflags.SET_ATIM_NOW | fstflags.SET_MTIM_NOW
      ) === errno.SUCCESS
    );
  }

  /**
   * Return the directory associated to that descriptor
   */
  dirName(): String {
    let path_max: usize = 4096;
    for (; ;) {
      let path_buf = changetype<usize>(new ArrayBuffer(path_max));
      let ret = fd_prestat_dir_name(this.rawfd, path_buf, path_max);
      if (ret === errno.NAMETOOLONG) {
        path_max = path_max * 2;
        continue;
      }
      let path_len = 0;
      while (load<u8>(path_buf + path_len) !== 0) {
        path_len++;
      }
      return String.UTF8.decodeUnsafe(path_buf, path_len);
    }
  }

  /**
   * Close a file descriptor
   */
  close(): void {
    fd_close(this.rawfd);
  }

  /**
   * Write data to a file descriptor
   * @param data data
   */
  write(data: Array<u8>): void {
    let data_buf_len = data.length;
    let data_buf = changetype<usize>(new ArrayBuffer(data_buf_len));
    for (let i = 0; i < data_buf_len; i++) {
      store<u8>(data_buf + i, unchecked(data[i]));
    }
    let iov = changetype<usize>(new ArrayBuffer(2 * sizeof<usize>()));
    store<u32>(iov, data_buf);
    store<u32>(iov + sizeof<usize>(), data_buf_len);

    let written_ptr = changetype<usize>(new ArrayBuffer(sizeof<usize>()));
    fd_write(this.rawfd, iov, 1, written_ptr);
  }

  /**
     * Write a string to a file descriptor, after encoding it to UTF8
     * @param s string
     * @param newline `true` to add a newline after the string
     */
  writeString(s: string, newline: bool = false): void {
    if (newline) {
      this.writeStringLn(s);
      return;
    }
    let s_utf8_len: usize = String.UTF8.byteLength(s);
    let s_utf8 = changetype<usize>(String.UTF8.encode(s));
    let iov = changetype<usize>(new ArrayBuffer(2 * sizeof<usize>()));
    store<u32>(iov, s_utf8);
    store<u32>(iov + sizeof<usize>(), s_utf8_len);
    let written_ptr = changetype<usize>(new ArrayBuffer(sizeof<usize>()));
    fd_write(this.rawfd, iov, 1, written_ptr);
  }

  /**
   * Write a string to a file descriptor, after encoding it to UTF8, with a newline
   * @param s string
   */
  writeStringLn(s: string): void {
    let s_utf8_len: usize = String.UTF8.byteLength(s);
    let s_utf8 = changetype<usize>(String.UTF8.encode(s));
    let iov = changetype<usize>(new ArrayBuffer(4 * sizeof<usize>()));
    store<u32>(iov, s_utf8);
    store<u32>(iov + sizeof<usize>(), s_utf8_len);
    let lf = changetype<usize>(new ArrayBuffer(1));
    store<u8>(lf, 10);
    store<u32>(iov + sizeof<usize>() * 2, lf);
    store<u32>(iov + sizeof<usize>() * 3, 1);
    let written_ptr = changetype<usize>(new ArrayBuffer(sizeof<usize>()));
    fd_write(this.rawfd, iov, 2, written_ptr);
  }

  /**
   * Read data from a file descriptor
   * @param data existing array to push data to
   * @param chunk_size chunk size (default: 4096)
   */
  read(
    data: Array<u8> = [],
    chunk_size: usize = 4096
  ): Array<u8> | null {
    let data_partial_len = chunk_size;
    let data_partial = changetype<usize>(new ArrayBuffer(data_partial_len));
    let iov = changetype<usize>(new ArrayBuffer(2 * sizeof<usize>()));
    store<u32>(iov, data_partial);
    store<u32>(iov + sizeof<usize>(), data_partial_len);
    let read_ptr = changetype<usize>(new ArrayBuffer(sizeof<usize>()));
    fd_read(this.rawfd, iov, 1, read_ptr);
    let read = load<usize>(read_ptr);
    if (read > 0) {
      for (let i: usize = 0; i < read; i++) {
        data.push(load<u8>(data_partial + i));
      }
    }
    if (read <= 0) {
      return null;
    }
    return data;
  }

  /**
   * Read from a file descriptor until the end of the stream
   * @param data existing array to push data to
   * @param chunk_size chunk size (default: 4096)
   */
  readAll(
    data: Array<u8> = [],
    chunk_size: usize = 4096
  ): Array<u8> | null {
    let data_partial_len = chunk_size;
    let data_partial = changetype<usize>(new ArrayBuffer(data_partial_len));
    let iov = changetype<usize>(new ArrayBuffer(2 * sizeof<usize>()));
    store<u32>(iov, data_partial);
    store<u32>(iov + sizeof<usize>(), data_partial_len);
    let read_ptr = changetype<usize>(new ArrayBuffer(sizeof<usize>()));
    let read: usize = 0;
    for (; ;) {
      if (fd_read(this.rawfd, iov, 1, read_ptr) !== errno.SUCCESS) {
        break;
      }
      read = load<usize>(read_ptr);
      if (read <= 0) {
        break;
      }
      for (let i: usize = 0; i < read; i++) {
        data.push(load<u8>(data_partial + i));
      }
    }
    if (read < 0) {
      return null;
    }
    return data;
  }

  /**
   * Read an UTF8 string from a file descriptor, convert it to a native string
   * @param chunk_size chunk size (default: 4096)
   */
  readString(chunk_size: usize = 4096): string | null {
    let s_utf8 = this.readAll();
    if (s_utf8 === null) {
      return null;
    }
    let s_utf8_len = s_utf8.length;
    let s_utf8_buf = changetype<usize>(new ArrayBuffer(s_utf8_len));
    for (let i = 0; i < s_utf8_len; i++) {
      store<u8>(s_utf8_buf + i, s_utf8[i]);
    }
    let s = String.UTF8.decodeUnsafe(s_utf8_buf, s_utf8.length);

    return s;
  }

  /**
   * Seek into a stream
   * @off offset
   * @w the position relative to which to set the offset of the file descriptor.
   */
  seek(off: u64, w: whence): bool {
    let fodder = changetype<usize>(new ArrayBuffer(8));
    let res = fd_seek(this.rawfd, off, w, fodder);

    return res === errno.SUCCESS;
  }

  /**
   * Return the current offset in the stream
   * @returns offset
   */
  tell(): u64 {
    let buf_off = changetype<usize>(new ArrayBuffer(8));
    let res = fd_tell(this.rawfd, buf_off);
    if (res !== errno.SUCCESS) {
      abort();
    }
    return load<u64>(buf_off);
  }
}

/**
 * A class to access a filesystem
 */
export class FileSystem {
  /**
   * Open a path
   * @path path
   * @flags r, r+, w, wx, w+ or xw+
   * @returns a descriptor
   */
  static open(path: string, flags: string = "r"): Descriptor | null {
    let dirfd = this.dirfdForPath(path);
    let fd_lookup_flags = lookupflags.SYMLINK_FOLLOW;
    let fd_oflags: u16 = 0;
    let fd_rights: u64 = 0;
    if (flags === "r") {
      fd_rights = rights.FD_READ | rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.FD_READDIR;
    } else if (flags === "r+") {
      fd_rights =
        rights.FD_READ | rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.FD_WRITE |
        rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.PATH_CREATE_FILE;
    } else if (flags === "w") {
      fd_oflags = oflags.CREAT | oflags.TRUNC;
      fd_rights = rights.FD_WRITE | rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.PATH_CREATE_FILE;
    } else if (flags === "wx") {
      fd_oflags = oflags.CREAT | oflags.TRUNC | oflags.EXCL;
      fd_rights = rights.FD_WRITE | rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.PATH_CREATE_FILE;
    } else if (flags === "w+") {
      fd_oflags = oflags.CREAT | oflags.TRUNC;
      fd_rights =
        rights.FD_READ | rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.FD_WRITE |
        rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.PATH_CREATE_FILE;
    } else if (flags === "xw+") {
      fd_oflags = oflags.CREAT | oflags.TRUNC | oflags.EXCL;
      fd_rights =
        rights.FD_READ | rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.FD_WRITE |
        rights.FD_SEEK | rights.FD_TELL | rights.FD_FILESTAT_GET | rights.PATH_CREATE_FILE;
    } else {
      return null;
    }
    let fd_rights_inherited = fd_rights;
    let fd_flags: fdflags = 0;
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let fd_buf = changetype<usize>(new ArrayBuffer(sizeof<u32>()));
    let res = path_open(
      dirfd as fd,
      fd_lookup_flags,
      path_utf8,
      path_utf8_len,
      fd_oflags,
      fd_rights,
      fd_rights_inherited,
      fd_flags,
      fd_buf
    );
    if (res !== errno.SUCCESS) {
      return null;
    }
    let fd = load<u32>(fd_buf);

    return new Descriptor(fd);
  }

  /**
   * Create a new directory
   * @path path
   * @returns `true` on success, `false` on failure
   */
  static mkdir(path: string): bool {
    let dirfd = this.dirfdForPath(path);
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let res = path_create_directory(dirfd, path_utf8, path_utf8_len);

    return res === errno.SUCCESS;
  }

  /**
   * Check if a file exists at a given path
   * @path path
   * @returns `true` on success, `false` on failure
   */
  static exists(path: string): bool {
    let dirfd = this.dirfdForPath(path);
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let fd_lookup_flags = lookupflags.SYMLINK_FOLLOW;
    let st_buf = changetype<usize>(new ArrayBuffer(56));
    let res = path_filestat_get(dirfd, fd_lookup_flags, path_utf8, path_utf8_len,
      changetype<filestat>(st_buf));

    return res === errno.SUCCESS;
  }

  /**
   * Create a hard link
   * @old_path old path
   * @new_path new path
   * @returns `true` on success, `false` on failure
   */
  static link(old_path: string, new_path: string): bool {
    let old_dirfd = this.dirfdForPath(old_path);
    let old_path_utf8_len: usize = String.UTF8.byteLength(old_path);
    let old_path_utf8 = changetype<usize>(String.UTF8.encode(old_path));
    let new_dirfd = this.dirfdForPath(new_path);
    let new_path_utf8_len: usize = String.UTF8.byteLength(new_path);
    let new_path_utf8 = changetype<usize>(String.UTF8.encode(new_path));
    let fd_lookup_flags = lookupflags.SYMLINK_FOLLOW;
    let res = path_link(old_dirfd, fd_lookup_flags, old_path_utf8, old_path_utf8_len,
      new_dirfd, new_path_utf8, new_path_utf8_len);

    return res === errno.SUCCESS;
  }

  /**
   * Create a symbolic link
   * @old_path old path
   * @new_path new path
   * @returns `true` on success, `false` on failure
   */
  static symlink(old_path: string, new_path: string): bool {
    let old_path_utf8_len: usize = String.UTF8.byteLength(old_path);
    let old_path_utf8 = changetype<usize>(String.UTF8.encode(old_path));
    let new_dirfd = this.dirfdForPath(new_path);
    let new_path_utf8_len: usize = String.UTF8.byteLength(new_path);
    let new_path_utf8 = changetype<usize>(String.UTF8.encode(new_path));
    let res = path_symlink(old_path_utf8, old_path_utf8_len,
      new_dirfd, new_path_utf8, new_path_utf8_len);

    return res === errno.SUCCESS;
  }

  /**
   * Unlink a file
   * @path path
   * @returns `true` on success, `false` on failure
   */
  static unlink(path: string): bool {
    let dirfd = this.dirfdForPath(path);
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let res = path_unlink_file(dirfd, path_utf8, path_utf8_len);

    return res === errno.SUCCESS;
  }

  /**
   * Remove a directory
   * @path path
   * @returns `true` on success, `false` on failure
   */
  static rmdir(path: string): bool {
    let dirfd = this.dirfdForPath(path);
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let res = path_remove_directory(dirfd, path_utf8, path_utf8_len);

    return res === errno.SUCCESS;
  }

  /**
   * Retrieve information about a file
   * @path path
   * @returns a `FileStat` object
   */
  static stat(path: string): FileStat {
    let dirfd = this.dirfdForPath(path);
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let fd_lookup_flags = lookupflags.SYMLINK_FOLLOW;
    let st_buf = changetype<usize>(new ArrayBuffer(56));
    if (path_filestat_get(dirfd, fd_lookup_flags, path_utf8, path_utf8_len, changetype<filestat>(st_buf)) !== errno.SUCCESS) {
      throw new WASAError("Unable to get the file information");
    }
    return new FileStat(st_buf);
  }

  /**
   * Retrieve information about a file or a symbolic link
   * @path path
   * @returns a `FileStat` object
   */
  static lstat(path: string): FileStat {
    let dirfd = this.dirfdForPath(path);
    let path_utf8_len: usize = String.UTF8.byteLength(path);
    let path_utf8 = changetype<usize>(String.UTF8.encode(path));
    let fd_lookup_flags = 0;
    let st_buf = changetype<usize>(new ArrayBuffer(56));
    if (path_filestat_get(dirfd, fd_lookup_flags, path_utf8, path_utf8_len, changetype<filestat>(st_buf)) !== errno.SUCCESS) {
      throw new WASAError("Unable to get the file information");
    }
    return new FileStat(st_buf);
  }

  /**
   * Rename a file
   * @old_path old path
   * @new_path new path
   * @returns `true` on success, `false` on failure
   */
  static rename(old_path: string, new_path: string): bool {
    let old_dirfd = this.dirfdForPath(old_path);
    let old_path_utf8_len: usize = String.UTF8.byteLength(old_path);
    let old_path_utf8 = changetype<usize>(String.UTF8.encode(old_path));
    let new_dirfd = this.dirfdForPath(new_path);
    let new_path_utf8_len: usize = String.UTF8.byteLength(new_path);
    let new_path_utf8 = changetype<usize>(String.UTF8.encode(new_path));
    let res = path_rename(old_dirfd, old_path_utf8, old_path_utf8_len,
      new_dirfd, new_path_utf8, new_path_utf8_len);

    return res === errno.SUCCESS;
  }

  /**
   * Get the content of a directory
   * @param path the directory path
   * @returns An array of file names
   */
  static readdir(path: string): Array<string> | null {
    let fd = this.open(path, "r");
    if (fd === null) {
      return null;
    }
    let out = new Array<string>();
    let buf = null;
    let buf_size = 4096;
    let buf_used_p = changetype<usize>(new ArrayBuffer(4));
    let buf_used = 0;
    for (; ;) {
      buf = __alloc(buf_size, 0);
      if (fd_readdir(fd.rawfd, buf, buf_size, 0 as dircookie, buf_used_p) !== errno.SUCCESS) {
        fd.close();
      }
      buf_used = load<u32>(buf_used_p);
      if (buf_used < buf_size) {
        break;
      }
      buf_size <<= 1;
      __free(buf);
    }
    let offset = 0;
    while (offset < buf_used) {
      offset += 16;
      let name_len = load<u32>(buf + offset);
      offset += 8;
      if (offset + name_len > buf_used) {
        return null;
      }
      let name = String.UTF8.decodeUnsafe(buf + offset, name_len);
      out.push(name);
      offset += name_len;
    }
    __free(buf);
    fd.close();

    return out;
  }

  protected static dirfdForPath(path: string): fd {
    return 3;
  }
}

@global
export class Console {
  /**
   * Write a string to the console
   * @param s string
   * @param newline `false` to avoid inserting a newline after the string
   */
  static write(s: string, newline: bool = true): void {
    Descriptor.Stdout().writeString(s, newline);
  }

  /**
   * Read an UTF8 string from the console, convert it to a native string
   */
  static readAll(): string | null {
    return Descriptor.Stdin().readString();
  }

  /**
   * Alias for `Console.write()`
   */
  static log(s: string): void {
    this.write(s);
  }

  /**
   * Write an error to the console
   * @param s string
   * @param newline `false` to avoid inserting a newline after the string
   */
  static error(s: string, newline: bool = true): void {
    Descriptor.Stderr().writeString(s, newline);
  }
}

export class Random {
  /**
   * Fill a buffer with random data
   * @param buffer An array buffer
   */
  static randomFill(buffer: ArrayBuffer): void {
    let len = buffer.byteLength;
    let ptr = changetype<usize>(buffer);
    while (len > 0) {
      let chunk = min(len, 256);
      if (random_get(ptr, chunk) !== errno.SUCCESS) {
        abort();
      }
      len -= chunk;
      ptr += chunk;
    }
  }

  /**
   * Return an array of random bytes
   * @param len length
   */
  static randomBytes(len: usize): Uint8Array {
    let array = new Uint8Array(len);
    this.randomFill(array.buffer);
    return array;
  }
}

export class Date {
  /**
   * Return the current timestamp, as a number of milliseconds since the epoch
   */
  static now(): f64 {
    let time_ptr = changetype<usize>(new ArrayBuffer(8));
    clock_time_get(clockid.REALTIME, 1000, time_ptr);
    let unix_ts = load<u64>(time_ptr);

    return (unix_ts as f64) / 1000.0;
  }
}

export class Performance {
  static now(): f64 {
    let time_ptr = changetype<usize>(new ArrayBuffer(8));
    clock_res_get(clockid.MONOTONIC, time_ptr);
    let res_ts = load<u64>(time_ptr);

    return res_ts as f64;
  }
}

export class Process {
  /**
   * Cleanly terminate the current process
   * @param status exit code
   */
  static exit(status: u32): void {
    proc_exit(status);
  }
}

export class EnvironEntry {
  constructor(readonly key: string, readonly value: string) { }
}

export class Environ {
  env: Array<EnvironEntry>;

  constructor() {
    this.env = [];
    let count_and_size = changetype<usize>(
      new ArrayBuffer(2 * sizeof<usize>())
    );
    let ret = environ_sizes_get(count_and_size, count_and_size + 4);
    if (ret !== errno.SUCCESS) {
      abort();
    }
    let count = load<usize>(count_and_size);
    let size = load<usize>(count_and_size + sizeof<usize>());
    let env_ptrs = changetype<usize>(
      new ArrayBuffer((count + 1) * sizeof<usize>())
    );
    let buf = changetype<usize>(new ArrayBuffer(size));
    if (environ_get(env_ptrs, buf) !== errno.SUCCESS) {
      abort();
    }
    for (let i: usize = 0; i < count; i++) {
      let env_ptr = load<usize>(env_ptrs + i * sizeof<usize>());
      let env_ptr_split = StringUtils.fromCString(env_ptr).split("=", 2);
      let key = env_ptr_split[0];
      let value = env_ptr_split[1];
      this.env.push(new EnvironEntry(key, value));
    }
  }

  /**
   *  Return all environment variables
   */
  all(): Array<EnvironEntry> {
    return this.env;
  }

  /**
   * Return the value for an environment variable
   * @param key environment variable name
   */
  get(key: string): string | null {
    for (let i = 0, j = this.env.length; i < j; i++) {
      if (this.env[i].key === key) {
        return this.env[i].value;
      }
    }
    return null;
  }
}

export class CommandLine {
  args: Array<string>;

  constructor() {
    this.args = [];
    let count_and_size = changetype<usize>(
      new ArrayBuffer(2 * sizeof<usize>())
    );
    let ret = args_sizes_get(count_and_size, count_and_size + 4);
    if (ret !== errno.SUCCESS) {
      abort();
    }
    let count = load<usize>(count_and_size);
    let size = load<usize>(count_and_size + sizeof<usize>());
    let env_ptrs = changetype<usize>(
      new ArrayBuffer((count + 1) * sizeof<usize>())
    );
    let buf = changetype<usize>(new ArrayBuffer(size));
    if (args_get(env_ptrs, buf) !== errno.SUCCESS) {
      abort();
    }
    for (let i: usize = 0; i < count; i++) {
      let env_ptr = load<usize>(env_ptrs + i * sizeof<usize>());
      let arg = StringUtils.fromCString(env_ptr);
      this.args.push(arg);
    }
  }

  /**
   * Return all the command-line arguments
   */
  all(): Array<string> {
    return this.args;
  }

  /**
   * Return the i-th command-ine argument
   * @param i index
   */
  get(i: usize): string | null {
    let args_len: usize = this.args[0].length;
    if (i < args_len) {
      return this.args[i];
    }
    return null;
  }
}

class StringUtils {
  /**
   * Returns a native string from a zero-terminated C string
   * @param cstring
   * @returns native string
   */
  static fromCString(cstring: usize): string {
    let size = 0;
    while (load<u8>(cstring + size) !== 0) {
      size++;
    }
    return String.UTF8.decodeUnsafe(cstring, size);
  }
}

@global
export function wasi_abort(
  message: string | null = "",
  fileName: string | null = "",
  lineNumber: u32 = 0,
  columnNumber: u32 = 0
): void {
  Console.error(fileName! + ":" + lineNumber.toString() + ":" + columnNumber.toString() +
    ": error: " + message!);
  proc_exit(255);
}
