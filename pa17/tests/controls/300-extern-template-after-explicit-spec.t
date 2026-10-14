// N3485 14.7.2p5: an explicit instantiation declaration after an explicit
// specialization has no effect, so the TU still emits X<int>'s vague-linkage
// vtable; expect link ok and exit 0
template<class T> struct X;
template<> struct X<int> { virtual ~X() {} virtual int f() const { return 41; } };
extern template struct X<int>;
int main() { X<int> x; const X<int>& r = x; return r.f() == 41 ? 0 : 1; }
