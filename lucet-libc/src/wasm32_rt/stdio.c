
#include "stdio_impl.h"
#include <errno.h>
#include <stdint.h>

#include "lucet_libc.h"

size_t __stdio_write(FILE *f, const unsigned char *buf, size_t len)
{
	lucet_libc_stdio(f->fd, buf, len);
	return len;
}

size_t __stdout_write(FILE *f, const unsigned char *buf, size_t len)
{
	f->write = __stdio_write;
	return __stdio_write(f, buf, len);
}

size_t __stdio_read(FILE *f, unsigned char *buf, size_t len)
{
	(void) f; (void) buf; (void) len;
	return 0;
}

off_t __stdio_seek(FILE *f, off_t off, int whence)
{
	(void) f; (void) off; (void) whence;
	errno = EIO;
	return -1;
}

int __stdio_close(FILE *f) 
{
	(void) f;
	errno = EIO;
	return -1;
}
