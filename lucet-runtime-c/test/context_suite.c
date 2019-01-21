#include <inttypes.h>

#include "greatest.h"

#include "../src/lucet_context_private.h"
#include "lucet_instance.h"

static char output_string[1024];
static void output(const char *fmt, ...);
static void reset_output(void);

static struct lucet_context parent_regs;
static struct lucet_context child_regs;

void arg_printing_child(void *arg0, void *arg1)
{
    int arg0_val = *(int *) arg0;
    int arg1_val = *(int *) arg1;

    output("hello from the child! my args were %d and %d\n", arg0_val, arg1_val);

    lucet_context_swap(&child_regs, &parent_regs);

    // Read the arguments again
    arg0_val = *(int *) arg0;
    arg1_val = *(int *) arg1;

    output("now they are %d and %d\n", arg0_val, arg1_val);

    lucet_context_swap(&child_regs, &parent_regs);
}

TEST call_child(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    int arg0 = 123;
    int arg1 = 456;

    lucet_context_init(&child_regs, stack_top, &parent_regs, arg_printing_child, &arg0, 1,
                       LUCET_VAL_C_PTR(&arg1));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("hello from the child! my args were 123 and 456\n", output_string);

    free(stack);

    PASS();
}

TEST call_child_twice(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    int arg0 = 123;
    int arg1 = 456;

    lucet_context_init(&child_regs, stack_top, &parent_regs, arg_printing_child, &arg0, 1,
                       LUCET_VAL_C_PTR(&arg1));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("hello from the child! my args were 123 and 456\n", output_string);

    arg0 = 9;
    arg1 = 10;

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ(
        "hello from the child! my args were 123 and 456\n"
        "now they are 9 and 10\n",
        output_string);

    free(stack);

    PASS();
}

// Use the lucet_context_set function to jump to the parent without saving
// the child
void context_set_child(void *vmctx)
{
    (void) vmctx;
    output("hello from the child! setting context to parent...\n");
    lucet_context_set(&parent_regs);
}

TEST call_child_setcontext(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, context_set_child, NULL, 0);

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("hello from the child! setting context to parent...\n", output_string);

    free(stack);

    PASS();
}

TEST call_child_setcontext_twice(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, context_set_child, NULL, 0);

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("hello from the child! setting context to parent...\n", output_string);

    lucet_context_init(&child_regs, stack_top, &parent_regs, context_set_child, NULL, 0);

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ(
        "hello from the child! setting context to parent...\n"
        "hello from the child! setting context to parent...\n",
        output_string);

    free(stack);

    PASS();
}

void returning_child(void *vmctx)
{
    (void) vmctx;
    output("hello from the child! returning...\n");
}

TEST call_returning_child(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, returning_child, NULL, 0);

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("hello from the child! returning...\n", output_string);

    free(stack);

    PASS();
}

void child_3_args(uint64_t arg1, uint64_t arg2, uint64_t arg3)
{
    output("the good three args boy %" PRId64 " %" PRId64 " %" PRId64 "\n", arg1, arg2, arg3);
}
TEST test_child_3_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_3_args, (void *) ((uint64_t) 10),
                       2, LUCET_VAL_U64(11), LUCET_VAL_U64(12));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("the good three args boy 10 11 12\n", output_string);

    free(stack);

    PASS();
}

void child_4_args(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4)
{
    output("the large four args boy %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64 "\n", arg1, arg2,
           arg3, arg4);
}

TEST test_child_4_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_4_args, (void *) ((uint64_t) 20),
                       3, LUCET_VAL_U64(21), LUCET_VAL_U64(22), LUCET_VAL_U64(23));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("the large four args boy 20 21 22 23\n", output_string);

    free(stack);

    PASS();
}

void child_5_args(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5)
{
    output("the big five args son %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64 "\n",
           arg1, arg2, arg3, arg4, arg5);
}

TEST test_child_5_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_5_args, (void *) ((uint64_t) 30),
                       4, LUCET_VAL_U64(31), LUCET_VAL_U64(32), LUCET_VAL_U64(33),
                       LUCET_VAL_U64(34));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("the big five args son 30 31 32 33 34\n", output_string);

    free(stack);

    PASS();
}

void child_6_args(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5,
                  uint64_t arg6)
{
    output("6 args, hahaha long boy %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64
           " %" PRId64 "\n",
           arg1, arg2, arg3, arg4, arg5, arg6);
}

TEST test_child_6_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_6_args, (void *) ((uint64_t) 40),
                       5, LUCET_VAL_U64(41), LUCET_VAL_U64(42), LUCET_VAL_U64(43),
                       LUCET_VAL_U64(44), LUCET_VAL_U64(45));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("6 args, hahaha long boy 40 41 42 43 44 45\n", output_string);

    free(stack);

    PASS();
}

void child_7_args(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5,
                  uint64_t arg6, uint64_t arg7)
{
    output("7 args, hahaha long boy %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64
           " %" PRId64 " %" PRId64 "\n",
           arg1, arg2, arg3, arg4, arg5, arg6, arg7);
}

TEST test_child_7_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_7_args, (void *) ((uint64_t) 50),
                       6, LUCET_VAL_U64(51), LUCET_VAL_U64(52), LUCET_VAL_U64(53),
                       LUCET_VAL_U64(54), LUCET_VAL_U64(55), LUCET_VAL_U64(56));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("7 args, hahaha long boy 50 51 52 53 54 55 56\n", output_string);

    free(stack);

    PASS();
}

void child_8_args(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5,
                  uint64_t arg6, uint64_t arg7, uint64_t arg8)
{
    output("8 args, hahaha long boy %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64 " %" PRId64
           " %" PRId64 " %" PRId64 " %" PRId64 "\n",
           arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
}

TEST test_child_8_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_8_args, (void *) ((uint64_t) 60),
                       7, LUCET_VAL_U64(61), LUCET_VAL_U64(62), LUCET_VAL_U64(63),
                       LUCET_VAL_U64(64), LUCET_VAL_U64(65), LUCET_VAL_U64(66), LUCET_VAL_U64(67));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("8 args, hahaha long boy 60 61 62 63 64 65 66 67\n", output_string);

    free(stack);

    PASS();
}

void child_invalid_unsigned_args(uint64_t arg1, uint64_t arg2, uint64_t arg3)
{
    output("Unexpected call with arguments %" PRId64 " %" PRId64 " %" PRId64 "\n", arg1, arg2,
           arg3);
}
TEST test_child_invalid_unsigned_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    int ret = lucet_context_init(&child_regs, stack_top, &parent_regs, child_invalid_unsigned_args,
                                 (void *) ((uint64_t) 70), 2, LUCET_VAL_U8(256), LUCET_VAL_I32(0));

    ASSERT_EQ(ret, -1);

    free(stack);

    PASS();
}

void child_invalid_signed_args(uint64_t arg1, uint64_t arg2, uint64_t arg3)
{
    output("Unexpected call with arguments %" PRId64 " %" PRId64 " %" PRId64 "\n", arg1, arg2,
           arg3);
}
TEST test_child_invalid_signed_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    int ret =
        lucet_context_init(&child_regs, stack_top, &parent_regs, child_invalid_signed_args,
                           (void *) ((uint64_t) 70), 2, LUCET_VAL_U8(0), LUCET_VAL_I16(-65536));

    ASSERT_EQ(ret, -1);

    free(stack);

    PASS();
}

void child_invalid_bool_args(uint64_t arg1, uint64_t arg2, uint64_t arg3)
{
    output("Unexpected call with arguments %" PRId64 " %" PRId64 " %" PRId64 "\n", arg1, arg2,
           arg3);
}
TEST test_child_invalid_bool_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    int ret = lucet_context_init(&child_regs, stack_top, &parent_regs, child_invalid_bool_args,
                                 (void *) ((uint64_t) 70), 2, LUCET_VAL_BOOL(1), LUCET_VAL_BOOL(2));

    ASSERT_EQ(ret, -1);

    free(stack);

    PASS();
}

void child_7fp_args(uint64_t vm_ctx, double arg1, double arg2, double arg3, double arg4,
                    double arg5, double arg6, double arg7)
{
    (void) vm_ctx;
    output("7 args, hahaha floaty boy %.1f %.1f %.1f %.1f %.1f %.1f %.1f\n", arg1, arg2, arg3, arg4,
           arg5, arg6, arg7);
}

TEST test_child_7fp_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_7fp_args,
                       (void *) ((uint64_t) 60), 7, LUCET_VAL_F64(61.0), LUCET_VAL_F64(62.0),
                       LUCET_VAL_F64(63.0), LUCET_VAL_F64(64.0), LUCET_VAL_F64(65.0),
                       LUCET_VAL_F64(66.0), LUCET_VAL_F64(67.0));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("7 args, hahaha floaty boy 61.0 62.0 63.0 64.0 65.0 66.0 67.0\n", output_string);

    free(stack);

    PASS();
}

void child_8fp_args(uint64_t vm_ctx, double arg1, double arg2, double arg3, double arg4,
                    double arg5, double arg6, double arg7, double arg8)
{
    (void) vm_ctx;
    output("8 args, hahaha floaty boy %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f\n", arg1, arg2, arg3,
           arg4, arg5, arg6, arg7, arg8);
}

TEST test_child_8fp_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_8fp_args,
                       (void *) ((uint64_t) 60), 8, LUCET_VAL_F64(61.0), LUCET_VAL_F64(62.0),
                       LUCET_VAL_F64(63.0), LUCET_VAL_F64(64.0), LUCET_VAL_F64(65.0),
                       LUCET_VAL_F64(66.0), LUCET_VAL_F64(67.0), LUCET_VAL_F64(68.0));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("8 args, hahaha floaty boy 61.0 62.0 63.0 64.0 65.0 66.0 67.0 68.0\n",
                  output_string);

    free(stack);

    PASS();
}

void child_9fp_args(uint64_t vm_ctx, double arg1, double arg2, double arg3, double arg4,
                    double arg5, double arg6, double arg7, double arg8, double arg9)
{
    (void) vm_ctx;
    output("9 args, hahaha floaty boy %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f\n", arg1, arg2,
           arg3, arg4, arg5, arg6, arg7, arg8, arg9);
}

TEST test_child_9fp_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_9fp_args,
                       (void *) ((uint64_t) 60), 9, LUCET_VAL_F64(61.0), LUCET_VAL_F64(62.0),
                       LUCET_VAL_F64(63.0), LUCET_VAL_F64(64.0), LUCET_VAL_F64(65.0),
                       LUCET_VAL_F64(66.0), LUCET_VAL_F64(67.0), LUCET_VAL_F64(68.0),
                       LUCET_VAL_F64(69.0));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("9 args, hahaha floaty boy 61.0 62.0 63.0 64.0 65.0 66.0 67.0 68.0 69.0\n",
                  output_string);

    free(stack);

    PASS();
}

void child_10fp_args(uint64_t vm_ctx, double arg1, double arg2, double arg3, double arg4,
                     double arg5, double arg6, double arg7, double arg8, double arg9, double arg10)
{
    (void) vm_ctx;
    output("10 args, hahaha floaty boy %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f\n", arg1,
           arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10);
}

TEST test_child_10fp_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_10fp_args,
                       (void *) ((uint64_t) 60), 10, LUCET_VAL_F64(61.0), LUCET_VAL_F64(62.0),
                       LUCET_VAL_F64(63.0), LUCET_VAL_F64(64.0), LUCET_VAL_F64(65.0),
                       LUCET_VAL_F64(66.0), LUCET_VAL_F64(67.0), LUCET_VAL_F64(68.0),
                       LUCET_VAL_F64(69.0), LUCET_VAL_F64(69.1));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ("10 args, hahaha floaty boy 61.0 62.0 63.0 64.0 65.0 66.0 67.0 68.0 69.0 69.1\n",
                  output_string);

    free(stack);

    PASS();
}

void child_11fp_args(uint64_t vm_ctx, double arg1, double arg2, double arg3, double arg4,
                     double arg5, double arg6, double arg7, double arg8, double arg9, double arg10,
                     double arg11)
{
    (void) vm_ctx;
    output("11 args, hahaha floaty boy %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f %.1f\n",
           arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11);
}

TEST test_child_11fp_args(void)
{
    reset_output();

    char *stack     = calloc(4096, 1);
    char *stack_top = &stack[4096];

    lucet_context_init(&child_regs, stack_top, &parent_regs, child_11fp_args,
                       (void *) ((uint64_t) 60), 11, LUCET_VAL_F64(61.0), LUCET_VAL_F64(62.0),
                       LUCET_VAL_F64(63.0), LUCET_VAL_F64(64.0), LUCET_VAL_F64(65.0),
                       LUCET_VAL_F64(66.0), LUCET_VAL_F64(67.0), LUCET_VAL_F64(68.0),
                       LUCET_VAL_F64(69.0), LUCET_VAL_F64(69.1), LUCET_VAL_F64(69.2));

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_STR_EQ(
        "11 args, hahaha floaty boy 61.0 62.0 63.0 64.0 65.0 66.0 67.0 68.0 69.0 69.1 69.2\n",
        output_string);

    free(stack);

    PASS();
}

SUITE(context_suite)
{
    RUN_TEST(call_child);
    RUN_TEST(call_child_twice);
    RUN_TEST(call_child_setcontext);
    RUN_TEST(call_child_setcontext_twice);
    RUN_TEST(call_returning_child);

    RUN_TEST(test_child_3_args);
    RUN_TEST(test_child_4_args);
    RUN_TEST(test_child_5_args);
    RUN_TEST(test_child_6_args);
    RUN_TEST(test_child_7_args);
    RUN_TEST(test_child_8_args);

    RUN_TEST(test_child_invalid_unsigned_args);
    RUN_TEST(test_child_invalid_signed_args);
    RUN_TEST(test_child_invalid_bool_args);

    RUN_TEST(test_child_7fp_args);
    RUN_TEST(test_child_8fp_args);
    RUN_TEST(test_child_9fp_args);
    RUN_TEST(test_child_10fp_args);
    RUN_TEST(test_child_11fp_args);
}

// Helpers:
static char * output_cursor;
static size_t output_cursor_len;

static void reset_output(void)
{
    memset(output_string, 0, sizeof(output_string));
    output_cursor     = output_string;
    output_cursor_len = sizeof(output_string);
}

static void output(const char *fmt, ...)
{
    va_list args;
    va_start(args, fmt);
    int res = vsnprintf(output_cursor, output_cursor_len, fmt, args);
    if (res > 0) {
        output_cursor += res;
        output_cursor_len -= res;
    } else {
        abort();
    }
    va_end(args);
}
