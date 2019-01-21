#ifndef LUCET_STATS_PRIVATE_H
#define LUCET_STATS_PRIVATE_H

#include "lucet_stats.h"

// The global callback used for all stats updates.
extern lucet_stats_callback_t lucet_stats_callback;

// Called within liblucet for all stats emissions.
void lucet_stats_update(enum lucet_stat_type stat_type, int64_t value);

#endif // LUCET_STATS_PRIVATE_H
