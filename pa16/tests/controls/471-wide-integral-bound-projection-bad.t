// N3485 5.19; supported __int128 extension.
// expect reject
int values[__uint128_t(1) << 100];
