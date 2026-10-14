// N3485 14.7.2p4, 14.6.4.1p8: only a declaration of a function template must
// precede its explicit instantiation; the definition may follow; expect exit 0
template <class T> int g(T);
template int g(int);
template <class T> int g(T) { return 3; }
int main() { return g(1) == 3 ? 0 : 1; }
