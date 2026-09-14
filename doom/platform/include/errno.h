/* mini errno. one global, a couple of constants, nobody checks. */

#ifndef ROZE_ERRNO_H
#define ROZE_ERRNO_H

extern int errno;

#define ENOENT 2
#define EACCES 13
#define EISDIR 21

#endif
