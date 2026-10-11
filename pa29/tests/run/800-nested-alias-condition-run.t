// N3485 14.5.6.1p5-p6, 14.5.7, Itanium C++ ABI 5.1.5: two units define
// distinct function templates f whose return types differ in a condition
// reached through a nested member alias template (an alias of an alias with a
// defaulted parameter); each specialization's mangled name keeps its own
// condition (Xsr2geIiT_EE5valueE, as Clang mangles it), so the units' f<int>
// stay distinct
