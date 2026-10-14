// N3485 14.5.3p4-p5, 15.4p1: a member function of a class template may use its
// function parameter pack (declared from the class pack A...) in its
// noexcept-specification: noexcept(noexcept(g(a...))); expect exit 0
int g(int x) { return x; }
template<class... A> struct fn { int safe(A... a) noexcept(noexcept(g(a...))) { return g(a...); } };
int main() { fn<int> h; return h.safe(4) == 4 ? 0 : 1; }
