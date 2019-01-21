#ifndef LUCET_STATS_H
#define LUCET_STATS_H

#include <stdint.h>

#include "lucet_export.h"

// Enumeration representing all possible stats that liblucet will emit.
enum lucet_stat_type {
    lucet_stat_program_load = 0,
    lucet_stat_program_load_fail,
    lucet_stat_program_unload,
    lucet_stat_instantiate,
    lucet_stat_instantiate_fail,
    lucet_stat_run,
    lucet_stat_run_start,
    lucet_stat_exit_ok,
    lucet_stat_exit_error,
    lucet_stat_release_instance,
};

// lucet_stats_callback_t is the type sig that the embedder-provided callback is
// expected to match. Note that all stats are expected to emit values
// representable by int64_t.
typedef void (*lucet_stats_callback_t)(enum lucet_stat_type stat_type, int64_t value);

// Updates the global callback used to send stats to embedder.
void lucet_stats_set_callback(lucet_stats_callback_t cb) EXPORTED;

#endif // LUCET_STATS_H
