// N3485 14.7.3p13 (as g++/clang extend it to variable templates): template<>
// int w<int>; without initializer is a valid (zero-initialized) explicit
// specialization; expect exit 0
template <class T> int w = 3;
template <> int w<int>;
int main() { w<int> += 2; return w<int> == 2 ? 0 : 1; }
