// N3485 14.2p3, 3.3.2p1: inside its own definition a function template's name
// is a template-name, so r<N - 1>() is a template-id call; expect exit 0
template<int N> int r() { return 2 * r<N - 1>(); }
template<> int r<0>() { return 1; }
int main() { return r<3>() == 8 ? 0 : 1; }
