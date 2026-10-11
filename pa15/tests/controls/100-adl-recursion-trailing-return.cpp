// N3485 14.6.4.2, 3.4.2: in depth(I<N>)'s trailing return type
// decltype(depth(I<N-1>())), ADL at the point of instantiation finds the
// template itself (global I); depth(I<1>()) is valid; expect exit 0
template<int N> struct I {};
int depth(I<0>) { return 0; }
template<int N> auto depth(I<N>) -> decltype(depth(I<N-1>())) { return 1 + depth(I<N-1>()); }
int main() { return depth(I<2>()) == 2 ? 0 : 1; }
