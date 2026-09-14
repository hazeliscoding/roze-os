/* assert routed into the kernel panic path. sha1.c is the only user. */

#ifndef ROZE_ASSERT_H
#define ROZE_ASSERT_H

_Noreturn void roze_assert_fail(const char *expr, const char *file, int line);

#define assert(x) \
    ((x) ? (void)0 : roze_assert_fail(#x, __FILE__, __LINE__))

#endif
