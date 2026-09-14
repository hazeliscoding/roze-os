/* mini stdlib. allocation forwards to the rust kernel heap, exit is
   a kernel panic, getenv pretends the environment is empty. */

#ifndef ROZE_STDLIB_H
#define ROZE_STDLIB_H

#include <stddef.h>

void *malloc(size_t size);
void *calloc(size_t n, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);

_Noreturn void exit(int status);
_Noreturn void abort(void);

int atoi(const char *s);
double atof(const char *s);
int abs(int v);

char *getenv(const char *name);
int system(const char *cmd);

#endif
