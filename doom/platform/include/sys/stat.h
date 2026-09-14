/* sys/stat stub. m_misc wants mkdir, there is no filesystem to make
   directories in. */

#ifndef ROZE_SYS_STAT_H
#define ROZE_SYS_STAT_H

int mkdir(const char *path, unsigned mode);

#endif
