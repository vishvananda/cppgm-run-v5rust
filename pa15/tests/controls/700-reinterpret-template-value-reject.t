// expect reject
// N3485 5.19p2: reinterpret_cast is forbidden in constant evaluation.
template<int N> struct Value {};
Value<reinterpret_cast<int>(1)> value;
