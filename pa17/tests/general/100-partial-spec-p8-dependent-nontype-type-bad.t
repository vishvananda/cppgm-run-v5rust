// N3485 14.5.5p8 (second bullet, example C<T, 1>): the type of a specialized
// non-type argument shall not depend on a parameter of the specialization;
// expect reject
template<class T, T t> struct C {};
template<class T> struct C<T, 1> {};
int main() { return 0; }
