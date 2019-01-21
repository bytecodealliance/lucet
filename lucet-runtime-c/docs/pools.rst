*****
Pools
*****

Pools are contain empty allocations which are used to create new instances. A
new instance is created by requesting an allocation from a pool and converting
it for use with the specified module. When an instance is done being used, it
should be wiped and put back in the pool.

Pools are designed to be threadsafe.

API
===

.. include:: _build/lucet_pool.h.rst
