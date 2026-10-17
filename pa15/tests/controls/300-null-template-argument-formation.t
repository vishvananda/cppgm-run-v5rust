// N3485 14.1p4, 14.3.2p1,p5: canonical null arguments convert and bind
// independently of template declarations, defaults and forwarding; expect exit 0
typedef decltype(nullptr) null_type;
template<null_type N = nullptr> struct Null {
  static bool test() { return N == nullptr; }
};
template<class T, T N> struct Value {
  static bool test() { static_assert(N == nullptr, "null"); return N == nullptr; }
};
template<const int* P> bool pointer() { return P == nullptr; }
template<int* P = nullptr> bool forward() { return pointer<P>(); }
template<null_type N> int specialized() { return 1; }
template<> int specialized<nullptr>() { return 2; }
template<int* P> struct Box {};
template<int* P> bool deduced(Box<P>) { return P == nullptr; }
int main() {
  return Null<>::test() && Null<(nullptr)>::test()
      && Value<int*, nullptr>::test() && Value<null_type, nullptr>::test()
      && forward<>() && pointer<(int*)nullptr>()
      && deduced(Box<nullptr>()) && specialized<nullptr>() == 2 ? 0 : 1;
}
