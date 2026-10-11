// N3485 14.5.4p1, 11p2: only f<int> is a friend; f<char> naming private A::s
// is ill-formed; expect reject
template<class T> int f(T);
class A { static int s; friend int f<>(int); };
int A::s = 1;
template<class T> int f(T) { return A::s; }
int main() { return f(1) + f(0.0) - 2; }
