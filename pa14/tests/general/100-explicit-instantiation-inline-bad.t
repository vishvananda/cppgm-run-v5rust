// N3485 14.7.2p1: an explicit instantiation of a function template shall not
// use the inline or constexpr specifiers; expect reject
template<class T> int f(T) { return 0; }
template inline int f(int);
int main() { return 0; }
