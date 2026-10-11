// GNU __bf16 on x86-64 (GCC and Clang): converting to float is exact (a
// float's upper half), to double, long double, the integers and bool goes
// through float; converting from float, double and long double rounds once to
// nearest even, from an integer through float; arithmetic over every pair of a
// table of values (the infinities, a quiet NaN, subnormals, the largest finite
// values) gives GCC's and Clang's results
