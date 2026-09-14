/* mini stdio for the doom port. files are read only blobs the kernel
   registered (the wad), stdout and stderr are sentinels that feed the
   kernel log. no buffering, no seeking past what doom actually does. */

#ifndef ROZE_STDIO_H
#define ROZE_STDIO_H

#include <stddef.h>
#include <stdarg.h>

typedef struct FILE FILE;

extern FILE *stdout;
extern FILE *stderr;

#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

FILE *fopen(const char *path, const char *mode);
int fclose(FILE *f);
size_t fread(void *ptr, size_t size, size_t n, FILE *f);
size_t fwrite(const void *ptr, size_t size, size_t n, FILE *f);
int fseek(FILE *f, long off, int whence);
long ftell(FILE *f);
int feof(FILE *f);
int fflush(FILE *f);
int remove(const char *path);
int rename(const char *from, const char *to);

int printf(const char *fmt, ...);
int fprintf(FILE *f, const char *fmt, ...);
int vfprintf(FILE *f, const char *fmt, va_list ap);
int sprintf(char *dst, const char *fmt, ...);
int snprintf(char *dst, size_t n, const char *fmt, ...);
int vsnprintf(char *dst, size_t n, const char *fmt, va_list ap);
int sscanf(const char *src, const char *fmt, ...);
int fscanf(FILE *f, const char *fmt, ...);
int puts(const char *s);
int putchar(int c);

#endif
