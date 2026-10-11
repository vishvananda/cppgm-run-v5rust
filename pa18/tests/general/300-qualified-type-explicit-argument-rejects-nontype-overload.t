namespace n {
template<class A, int> void f(A) {}
template<class A, class> void f(A) {}
struct O {};
namespace op { template<class A> void g(A a) { f<A, n::O>(a); } }
}
int main() { n::op::g(0); }
