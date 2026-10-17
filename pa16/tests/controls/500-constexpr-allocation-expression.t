// N3485 5.19p2: allocation is not a core constant expression in C++11.
// expect reject
constexpr int* pointer = new int(3);
