// N3485 14.3.2p5: a pointer-to-member non-type template argument undergoes no
// conversions; &derived::f has type void (base::*)(), so check<derived,
// &derived::f> is a substitution failure; expect exit 0
struct base { void f() {} };
struct derived : base { };
template<class T, void (T::*)()> struct check {};
template<class T> char has(check<T, &T::f>*);
template<class> long has(...);
int main() { return sizeof(has<derived>(0)) == sizeof(long) && sizeof(has<base>(0)) == 1 ? 0 : 1; }
