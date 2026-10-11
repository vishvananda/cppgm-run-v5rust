// N3485 14.8.2p2 (bullet 2), 14.8.2p8: an explicit non-type argument that does
// not convert to the parameter type fails deduction for that template only;
// expect exit 0
template<int* P> int f() { return 1; }
template<int N> int f() { return 3; }
int main() { return f<5>() == 3 ? 0 : 1; }
