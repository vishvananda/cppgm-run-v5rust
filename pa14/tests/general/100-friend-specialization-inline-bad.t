// N3485 14.5.4p9: inline shall not be used in a friend declaration naming a
// function template specialization (g++ rejects, clang accepts); expect reject
template <class T> int f(T);
class A { friend inline int f<>(char); };
int main() { return 0; }
