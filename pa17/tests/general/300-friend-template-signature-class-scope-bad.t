// N3485 3.4.1p7, 14.5.6.1, 11.3: M in the friend template's signature is A::M
// (int), so the namespace-scope f using ::M (double) is a different template
// and not a friend; must be rejected (A::secret inaccessible)
typedef double M;
template<bool B> struct E {}; template<> struct E<true> { typedef int type; };
class A { typedef int M; static int secret; template<class T> friend typename E<sizeof(T) == sizeof(M)>::type f(T); };
int A::secret = 5;
template<class T> typename E<sizeof(T) == sizeof(M)>::type f(T) { return A::secret; }
int x = f(1.0);
int main() { return 0; }
