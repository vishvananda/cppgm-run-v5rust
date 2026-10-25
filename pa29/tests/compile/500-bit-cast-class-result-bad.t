struct S { unsigned value; };
S f(unsigned value) { return __builtin_bit_cast(S, value); }
