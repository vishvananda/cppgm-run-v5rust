// N3485 14.5.7p2, 14.5.6.1p5-p6: D<N> is I<N*2>, so both lines declare one
// function template; the call is not ambiguous; expect exit 0
template<int N> struct I {};
template<int N> using D = I<N * 2>;
template<int N> int f(D<N>);
template<int N> int f(I<N * 2>) { return N; }
int main() { return f<3>(I<6>()) == 3 ? 0 : 1; }
