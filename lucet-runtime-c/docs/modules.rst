*******
Modules
*******

In liblucet terms, a module is a dynamically linkable library matching a
specific ABI (described below). The most straightforward way to
generate one of these modules is by using lucetc.

Modules contain several different entities:

- Functions - These are normal SystemV ABI functions, which accept a
  VmCtx as their first argument. See Functions below.
- Tables - ...
- Data Initializers - ...
- Trap Manifest and Tables - ...

ABI
===

The ABI for modules used by liblucet is described below.

Function ABI
------------

...

Data Initializer ABI
--------------------

...

etc

API
===

.. include:: _build/lucet_module.h.rst
