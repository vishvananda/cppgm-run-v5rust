// PA29 README (builtins hosted headers use): __builtin_object_size(p, 0),
// which <istream>'s operator>>(istream&, char*) calls when __OPTIMIZE__ is
// defined, yields the object's size or (size_t)-1 and does not evaluate its
// operand
