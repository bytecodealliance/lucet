***********
Allocations
***********

An allocation (or lucet_alloc) is the base memory that is used for an
instance. These are generally created in advance and recycled after
being used.

Limits can be placed on allocations with the use of
lucet_alloc_limits. With this one can control the maximum stack, heap,
globals, and guard.

API
===

.. include:: _build/lucet_alloc.h.rst

