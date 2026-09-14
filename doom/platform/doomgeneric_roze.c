/* doomgeneric platform layer for rozeos.
   thin on purpose: every DG_ call forwards to a kernel export. the
   interesting work happens on the rust side of the fence. */

#include <stdint.h>

#include "doomgeneric.h"

/* kernel exports, kernel/src/doom/bindings.rs */
extern void roze_draw_frame(const uint32_t *frame, int width, int height);
extern uint32_t roze_ticks_ms(void);
extern void roze_sleep_ms(uint32_t ms);
extern int roze_get_key(int *pressed, unsigned char *key);
extern void roze_log_write(const char *buf, long len);

void DG_Init(void)
{
    /* screen buffer is allocated by doomgeneric_Create before this
       runs, kernel services were up long before. nothing to do. */
}

void DG_DrawFrame(void)
{
    roze_draw_frame(DG_ScreenBuffer, DOOMGENERIC_RESX, DOOMGENERIC_RESY);
}

void DG_SleepMs(uint32_t ms)
{
    roze_sleep_ms(ms);
}

uint32_t DG_GetTicksMs(void)
{
    return roze_ticks_ms();
}

int DG_GetKey(int *pressed, unsigned char *doomKey)
{
    return roze_get_key(pressed, doomKey);
}

void DG_SetWindowTitle(const char *title)
{
    /* no windows here, log it for flavor */
    roze_log_write("doom: ", 6);
    while (*title) {
        roze_log_write(title, 1);
        title++;
    }
    roze_log_write("\n", 1);
}
