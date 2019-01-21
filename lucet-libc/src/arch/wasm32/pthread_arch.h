extern struct pthread g_pthread_self;
static inline struct pthread *__pthread_self(void) { return &g_pthread_self; }

#define TP_ADJ(p) (p)

#define CANCEL_REG_IP 16
