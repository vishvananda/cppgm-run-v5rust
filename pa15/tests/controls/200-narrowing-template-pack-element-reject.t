// N3485 14.3.2p5: each non-type pack element converts without narrowing;
// expect reject
template<unsigned char... N>
struct bytes {};

bytes<1, 256> object;
