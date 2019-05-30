* idl package generates a lucetc::Bindings, binding info accessible in modules / func AST
* idl-test generates arbitrary values for a given type
* describe ABI of functions:
    * annotations to arguments saying whether they are in, out, or inout
    * non-atomic arguments need to be passed by reference, cannot be return values
* idl package generates funcs that take a byte array (linear memory) and offset
  and validate whether it is a valid repr of a type
    * pointer, length validation
    * enum variants
    * bools
* add byte slices to language.
    * hold off on other types of slices for now
