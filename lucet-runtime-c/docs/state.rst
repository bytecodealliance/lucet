*****
State
*****

Instances include detailed information describing the internal state
of the execution.

During normal operation this may just be a tag indicating that the
instance is initialized or running. However, during error events this
will contain detailed information about the signal that caused the
error, backtraces, and so on.

API
===

.. include:: _build/lucet_state.h.rst
