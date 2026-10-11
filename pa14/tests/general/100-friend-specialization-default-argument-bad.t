// N3485 14.5.4p9: a friend declaration naming a function template
// specialization shall not have default arguments; expect reject
template <class T> int f(T, int);
class A { friend int f<>(char, int = 3); };
int main() { return 0; }
