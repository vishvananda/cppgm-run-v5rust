// N3485 14.3.2p5: integer zero does not undergo null pointer conversion
// as a non-type template argument; expect reject
template<int* P> struct Pointer {};
Pointer<0> object;
