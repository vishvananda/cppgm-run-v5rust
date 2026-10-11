// VALIDATION: compile-pass
// N3485 focus: 3.4.2 [basic.lookup.argdep], 14.6 [temp.res]
namespace n {
struct a {};
a r(int &);
int f(a);
}
template<class> struct x { int i; int g() { return f(n::r(i)); } };
int main() {}
