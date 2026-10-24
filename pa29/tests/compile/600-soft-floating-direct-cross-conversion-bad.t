// PA29: bfloat16 and binary16 interconvert only as arithmetic operands,
// including casts in either direction: expect compile REJECT.
_Float16 convert(__bf16 x) { return static_cast<_Float16>(x); }
