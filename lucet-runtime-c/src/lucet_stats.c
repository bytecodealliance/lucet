#include <assert.h>
#include <stdint.h>
#include <stdlib.h>

#include "lucet_stats_private.h"

lucet_stats_callback_t lucet_stats_callback = NULL;

void lucet_stats_set_callback(lucet_stats_callback_t cb)
{
    lucet_stats_callback = cb;
}

void lucet_stats_update(enum lucet_stat_type stat_type, int64_t value)
{
    if (lucet_stats_callback) {
        lucet_stats_callback(stat_type, value);
    }
}
