/* the whole c library, rozeos edition.
   just enough libc to keep doomgeneric alive on bare metal. memory
   comes from the rust kernel heap, console output goes to the kernel
   log, and the filesystem is exactly one read only file: the wad the
   bootloader loaded. everything doom does not exercise is a stub that
   fails politely. */

#include <ctype.h>
#include <errno.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

/*=====================================================================
  kernel imports, implemented in rust (kernel/src/doom/bindings.rs)
=====================================================================*/

extern void *roze_malloc(size_t size);
extern void *roze_realloc(void *ptr, size_t size);
extern void roze_free(void *ptr);
extern void roze_log_write(const char *buf, long len);
extern _Noreturn void roze_exit(int status);
extern const unsigned char *roze_wad_data(long *len);

int errno;

/*=====================================================================
  memory
=====================================================================*/

void *malloc(size_t size)
{
    return roze_malloc(size);
}

void *calloc(size_t n, size_t size)
{
    size_t total = n * size;
    void *p = roze_malloc(total);
    if (p)
        memset(p, 0, total);
    return p;
}

void *realloc(void *ptr, size_t size)
{
    return roze_realloc(ptr, size);
}

void free(void *ptr)
{
    roze_free(ptr);
}

/*=====================================================================
  string.h
=====================================================================*/

void *memcpy(void *dst, const void *src, size_t n)
{
    unsigned char *d = dst;
    const unsigned char *s = src;
    while (n--)
        *d++ = *s++;
    return dst;
}

void *memmove(void *dst, const void *src, size_t n)
{
    unsigned char *d = dst;
    const unsigned char *s = src;
    if (d < s) {
        while (n--)
            *d++ = *s++;
    } else {
        d += n;
        s += n;
        while (n--)
            *--d = *--s;
    }
    return dst;
}

void *memset(void *dst, int c, size_t n)
{
    unsigned char *d = dst;
    while (n--)
        *d++ = (unsigned char)c;
    return dst;
}

int memcmp(const void *a, const void *b, size_t n)
{
    const unsigned char *x = a, *y = b;
    for (; n--; x++, y++)
        if (*x != *y)
            return *x - *y;
    return 0;
}

size_t strlen(const char *s)
{
    const char *p = s;
    while (*p)
        p++;
    return p - s;
}

int strcmp(const char *a, const char *b)
{
    while (*a && *a == *b)
        a++, b++;
    return (unsigned char)*a - (unsigned char)*b;
}

int strncmp(const char *a, const char *b, size_t n)
{
    while (n && *a && *a == *b)
        a++, b++, n--;
    return n ? (unsigned char)*a - (unsigned char)*b : 0;
}

char *strcpy(char *dst, const char *src)
{
    char *d = dst;
    while ((*d++ = *src++))
        ;
    return dst;
}

/* writes exactly n bytes, always. an early version decremented n one
   short of the copied nul and padded a 9th byte into 8 byte lump
   names, zeroing the low byte of the field behind them. doom noticed. */
char *strncpy(char *dst, const char *src, size_t n)
{
    char *d = dst;
    while (n && *src) {
        *d++ = *src++;
        n--;
    }
    while (n--)
        *d++ = 0;
    return dst;
}

char *strcat(char *dst, const char *src)
{
    strcpy(dst + strlen(dst), src);
    return dst;
}

char *strncat(char *dst, const char *src, size_t n)
{
    char *d = dst + strlen(dst);
    while (n-- && *src)
        *d++ = *src++;
    *d = 0;
    return dst;
}

char *strchr(const char *s, int c)
{
    for (;; s++) {
        if (*s == (char)c)
            return (char *)s;
        if (!*s)
            return 0;
    }
}

char *strrchr(const char *s, int c)
{
    const char *hit = 0;
    for (;; s++) {
        if (*s == (char)c)
            hit = s;
        if (!*s)
            return (char *)hit;
    }
}

char *strstr(const char *hay, const char *needle)
{
    size_t n = strlen(needle);
    if (!n)
        return (char *)hay;
    for (; *hay; hay++)
        if (!strncmp(hay, needle, n))
            return (char *)hay;
    return 0;
}

char *strdup(const char *s)
{
    size_t n = strlen(s) + 1;
    char *p = roze_malloc(n);
    if (p)
        memcpy(p, s, n);
    return p;
}

int strcasecmp(const char *a, const char *b)
{
    while (*a && tolower((unsigned char)*a) == tolower((unsigned char)*b))
        a++, b++;
    return tolower((unsigned char)*a) - tolower((unsigned char)*b);
}

int strncasecmp(const char *a, const char *b, size_t n)
{
    while (n && *a && tolower((unsigned char)*a) == tolower((unsigned char)*b))
        a++, b++, n--;
    return n ? tolower((unsigned char)*a) - tolower((unsigned char)*b) : 0;
}

/*=====================================================================
  ctype.h
=====================================================================*/

int toupper(int c)
{
    return (c >= 'a' && c <= 'z') ? c - 32 : c;
}

int tolower(int c)
{
    return (c >= 'A' && c <= 'Z') ? c + 32 : c;
}

int isspace(int c)
{
    return c == ' ' || (c >= '\t' && c <= '\r');
}

int isdigit(int c)
{
    return c >= '0' && c <= '9';
}

int isupper(int c)
{
    return c >= 'A' && c <= 'Z';
}

int islower(int c)
{
    return c >= 'a' && c <= 'z';
}

int isalpha(int c)
{
    return isupper(c) || islower(c);
}

int isalnum(int c)
{
    return isalpha(c) || isdigit(c);
}

int ispunct(int c)
{
    return c > ' ' && c < 0x7f && !isalnum(c);
}

/*=====================================================================
  stdlib misc
=====================================================================*/

int abs(int v)
{
    return v < 0 ? -v : v;
}

int atoi(const char *s)
{
    int v = 0, neg = 0;
    while (isspace((unsigned char)*s))
        s++;
    if (*s == '-')
        neg = 1, s++;
    else if (*s == '+')
        s++;
    while (isdigit((unsigned char)*s))
        v = v * 10 + (*s++ - '0');
    return neg ? -v : v;
}

double atof(const char *s)
{
    double v = 0, frac = 0, div = 1;
    int neg = 0;
    while (isspace((unsigned char)*s))
        s++;
    if (*s == '-')
        neg = 1, s++;
    else if (*s == '+')
        s++;
    while (isdigit((unsigned char)*s))
        v = v * 10 + (*s++ - '0');
    if (*s == '.') {
        s++;
        while (isdigit((unsigned char)*s)) {
            frac = frac * 10 + (*s++ - '0');
            div *= 10;
        }
    }
    v += frac / div;
    return neg ? -v : v;
}

char *getenv(const char *name)
{
    (void)name; /* no environment on the bare metal */
    return 0;
}

int system(const char *cmd)
{
    (void)cmd; /* no shell to hand this to */
    return -1;
}

_Noreturn void exit(int status)
{
    roze_exit(status);
}

_Noreturn void abort(void)
{
    roze_exit(134); /* 128 + sigabrt, for old times sake */
}

_Noreturn void roze_assert_fail(const char *expr, const char *file, int line)
{
    printf("assert failed: %s at %s:%d\n", expr, file, line);
    roze_exit(1);
}

/*=====================================================================
  math, the two calls that survive doom's fixed point diet
=====================================================================*/

double fabs(double x)
{
    return x < 0 ? -x : x;
}

/*=====================================================================
  files. the entire filesystem is the wad.
=====================================================================*/

struct FILE {
    const unsigned char *data;
    long len;
    long pos;
};

/* sentinel pointers for the console streams, never dereferenced */
static struct FILE stream_out;
static struct FILE stream_err;
FILE *stdout = &stream_out;
FILE *stderr = &stream_err;

static struct FILE wad_handle;
static int wad_open;

FILE *fopen(const char *path, const char *mode)
{
    long len;
    const unsigned char *data;
    const char *base;

    /* writes always fail, saves and configs come later milestones */
    if (strchr(mode, 'w') || strchr(mode, 'a'))
        return 0;

    data = roze_wad_data(&len);
    if (!data || wad_open)
        return 0;

    /* match on basename so "doom1.wad" and "./doom1.wad" both hit */
    base = strrchr(path, '/');
    base = base ? base + 1 : path;
    if (strcasecmp(base, "doom1.wad") != 0)
        return 0;

    wad_handle.data = data;
    wad_handle.len = len;
    wad_handle.pos = 0;
    wad_open = 1;
    return &wad_handle;
}

int fclose(FILE *f)
{
    if (f == &wad_handle)
        wad_open = 0;
    return 0;
}

size_t fread(void *ptr, size_t size, size_t n, FILE *f)
{
    long want, have;
    if (f != &wad_handle || size == 0)
        return 0;
    want = (long)(size * n);
    have = f->len - f->pos;
    if (want > have)
        want = have;
    if (want <= 0)
        return 0;
    memcpy(ptr, f->data + f->pos, (size_t)want);
    f->pos += want;
    return (size_t)want / size;
}

size_t fwrite(const void *ptr, size_t size, size_t n, FILE *f)
{
    /* console streams print, everything else swallows */
    if (f == stdout || f == stderr) {
        roze_log_write(ptr, (long)(size * n));
        return n;
    }
    return 0;
}

int fseek(FILE *f, long off, int whence)
{
    long base;
    if (f != &wad_handle)
        return -1;
    switch (whence) {
    case SEEK_SET: base = 0; break;
    case SEEK_CUR: base = f->pos; break;
    case SEEK_END: base = f->len; break;
    default: return -1;
    }
    if (base + off < 0 || base + off > f->len)
        return -1;
    f->pos = base + off;
    return 0;
}

long ftell(FILE *f)
{
    return f == &wad_handle ? f->pos : -1;
}

int feof(FILE *f)
{
    return f == &wad_handle ? f->pos >= f->len : 1;
}

int fflush(FILE *f)
{
    (void)f;
    return 0;
}

int remove(const char *path)
{
    (void)path;
    return -1;
}

int rename(const char *from, const char *to)
{
    (void)from;
    (void)to;
    return -1;
}

int mkdir(const char *path, unsigned mode)
{
    (void)path;
    (void)mode;
    return -1;
}

/*=====================================================================
  printf machinery. handles what doom formats: %s %c %d %i %u %x %X
  %p %% %f, flags - 0, field width, precision for strings and floats,
  l and ll length mods. no scientific notation, no locales, no tears.
=====================================================================*/

static void emit(char *dst, size_t cap, size_t *at, char c)
{
    if (*at + 1 < cap)
        dst[*at] = c;
    (*at)++;
}

static void emit_pad(char *dst, size_t cap, size_t *at, char c, int n)
{
    while (n-- > 0)
        emit(dst, cap, at, c);
}

static void emit_num(char *dst, size_t cap, size_t *at, unsigned long long v,
                     int base, int upper, int neg, int width, int zero,
                     int prec)
{
    char tmp[24];
    int n = 0;
    const char *digits = upper ? "0123456789ABCDEF" : "0123456789abcdef";

    do {
        tmp[n++] = digits[v % base];
        v /= base;
    } while (v);
    // precision on integers means minimum digits, "%.3d" of 33 is 033.
    // the hud font loader spells lump names this way
    while (n < prec && n < (int)sizeof tmp - 1)
        tmp[n++] = '0';
    if (neg)
        tmp[n++] = '-';

    emit_pad(dst, cap, at, zero ? '0' : ' ', width - n);
    while (n--)
        emit(dst, cap, at, tmp[n]);
}

int vsnprintf(char *dst, size_t cap, const char *fmt, va_list ap)
{
    size_t at = 0;

    for (; *fmt; fmt++) {
        int minus = 0, zero = 0, width = 0, prec = -1, longs = 0;

        if (*fmt != '%') {
            emit(dst, cap, &at, *fmt);
            continue;
        }
        fmt++;

        for (;; fmt++) {
            if (*fmt == '-')
                minus = 1;
            else if (*fmt == '0')
                zero = 1;
            else
                break;
        }
        while (isdigit((unsigned char)*fmt))
            width = width * 10 + (*fmt++ - '0');
        if (*fmt == '.') {
            fmt++;
            prec = 0;
            while (isdigit((unsigned char)*fmt))
                prec = prec * 10 + (*fmt++ - '0');
        }
        while (*fmt == 'l') {
            longs++;
            fmt++;
        }

        switch (*fmt) {
        case 's': {
            const char *s = va_arg(ap, const char *);
            int len, pad;
            if (!s)
                s = "(null)";
            len = (int)strlen(s);
            if (prec >= 0 && len > prec)
                len = prec;
            pad = width - len;
            if (!minus)
                emit_pad(dst, cap, &at, ' ', pad);
            while (len--)
                emit(dst, cap, &at, *s++);
            if (minus)
                emit_pad(dst, cap, &at, ' ', pad);
            break;
        }
        case 'c':
            emit_pad(dst, cap, &at, ' ', width - 1);
            emit(dst, cap, &at, (char)va_arg(ap, int));
            break;
        case 'd':
        case 'i': {
            long long v = longs >= 2 ? va_arg(ap, long long)
                        : longs == 1 ? va_arg(ap, long)
                                     : va_arg(ap, int);
            unsigned long long mag = v < 0 ? (unsigned long long)-v : (unsigned long long)v;
            emit_num(dst, cap, &at, mag, 10, 0, v < 0, width, zero, prec);
            break;
        }
        case 'u': {
            unsigned long long v = longs >= 2 ? va_arg(ap, unsigned long long)
                                 : longs == 1 ? va_arg(ap, unsigned long)
                                              : va_arg(ap, unsigned);
            emit_num(dst, cap, &at, v, 10, 0, 0, width, zero, prec);
            break;
        }
        case 'x':
        case 'X': {
            unsigned long long v = longs >= 2 ? va_arg(ap, unsigned long long)
                                 : longs == 1 ? va_arg(ap, unsigned long)
                                              : va_arg(ap, unsigned);
            emit_num(dst, cap, &at, v, 16, *fmt == 'X', 0, width, zero, prec);
            break;
        }
        case 'p':
            emit(dst, cap, &at, '0');
            emit(dst, cap, &at, 'x');
            emit_num(dst, cap, &at, (unsigned long long)(uintptr_t)va_arg(ap, void *),
                     16, 0, 0, 0, 0, 0);
            break;
        case 'f': {
            double v = va_arg(ap, double);
            long long ip;
            int digits = prec < 0 ? 6 : prec;
            if (v < 0) {
                emit(dst, cap, &at, '-');
                v = -v;
            }
            ip = (long long)v;
            emit_num(dst, cap, &at, (unsigned long long)ip, 10, 0, 0, 0, 0, 0);
            if (digits > 0) {
                emit(dst, cap, &at, '.');
                v -= (double)ip;
                while (digits--) {
                    v *= 10;
                    ip = (long long)v;
                    emit(dst, cap, &at, (char)('0' + ip));
                    v -= (double)ip;
                }
            }
            break;
        }
        case '%':
            emit(dst, cap, &at, '%');
            break;
        default:
            /* unknown verb, print it raw so the bug is visible */
            emit(dst, cap, &at, '%');
            emit(dst, cap, &at, *fmt);
            break;
        }
    }

    if (cap)
        dst[at < cap ? at : cap - 1] = 0;
    return (int)at;
}

int snprintf(char *dst, size_t n, const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = vsnprintf(dst, n, fmt, ap);
    va_end(ap);
    return r;
}

int sprintf(char *dst, const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = vsnprintf(dst, 0x7fffffff, fmt, ap);
    va_end(ap);
    return r;
}

/* format into a stack buffer and hand the kernel one line. 1 KiB is
   plenty, doom's chattiest line is the startup banner. */
static int log_vprintf(const char *fmt, va_list ap)
{
    char buf[1024];
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    if (n > (int)sizeof buf - 1)
        n = (int)sizeof buf - 1;
    roze_log_write(buf, n);
    return n;
}

int printf(const char *fmt, ...)
{
    va_list ap;
    int r;
    va_start(ap, fmt);
    r = log_vprintf(fmt, ap);
    va_end(ap);
    return r;
}

int fprintf(FILE *f, const char *fmt, ...)
{
    va_list ap;
    int r;
    (void)f; /* stdout, stderr, same log */
    va_start(ap, fmt);
    r = log_vprintf(fmt, ap);
    va_end(ap);
    return r;
}

int vfprintf(FILE *f, const char *fmt, va_list ap)
{
    (void)f;
    return log_vprintf(fmt, ap);
}

int puts(const char *s)
{
    roze_log_write(s, (long)strlen(s));
    roze_log_write("\n", 1);
    return 0;
}

int putchar(int c)
{
    char ch = (char)c;
    roze_log_write(&ch, 1);
    return c;
}

/*=====================================================================
  scanf corner. m_misc parses numbers with sscanf, the config loader
  runs fscanf against a file that can never open. minimal on purpose.
=====================================================================*/

int sscanf(const char *src, const char *fmt, ...)
{
    va_list ap;
    int matched = 0;

    va_start(ap, fmt);
    for (; *fmt && src; fmt++) {
        if (isspace((unsigned char)*fmt)) {
            while (isspace((unsigned char)*src))
                src++;
            continue;
        }
        if (*fmt != '%') {
            if (*src != *fmt)
                break;
            src++;
            continue;
        }
        fmt++;
        if (*fmt == 'i' || *fmt == 'd') {
            const char *start = src;
            int v = atoi(src);
            if (*src == '-' || *src == '+')
                src++;
            while (isdigit((unsigned char)*src))
                src++;
            if (src == start)
                break;
            *va_arg(ap, int *) = v;
            matched++;
        } else if (*fmt == 'x') {
            unsigned v = 0;
            const char *start = src;
            for (;; src++) {
                int c = tolower((unsigned char)*src);
                if (isdigit(c))
                    v = v * 16 + (c - '0');
                else if (c >= 'a' && c <= 'f')
                    v = v * 16 + (c - 'a' + 10);
                else
                    break;
            }
            if (src == start)
                break;
            *va_arg(ap, unsigned *) = v;
            matched++;
        } else {
            break; /* verb we do not speak */
        }
    }
    va_end(ap);
    return matched;
}

int fscanf(FILE *f, const char *fmt, ...)
{
    (void)f;
    (void)fmt;
    return EOF; /* the config file never opens, nothing to scan */
}
