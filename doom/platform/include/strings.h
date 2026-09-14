/* case insensitive compares, posix leftovers doom leans on. */

#ifndef ROZE_STRINGS_H
#define ROZE_STRINGS_H

#include <stddef.h>

int strcasecmp(const char *a, const char *b);
int strncasecmp(const char *a, const char *b, size_t n);

#endif
