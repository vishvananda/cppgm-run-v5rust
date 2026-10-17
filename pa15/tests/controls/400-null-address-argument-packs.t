// N3485 14.3.2p5, 14.5.3: canonical null pointer/member arguments; expect exit 0
template<int*... P> struct Pointers {
  static const unsigned long count = sizeof...(P);
};
template<class T, T*... P> struct Dependent {
  static const unsigned long count = sizeof...(P);
};
struct Owner {};
template<int Owner::*... P> struct Members {
  static const unsigned long count = sizeof...(P);
};
template<int Owner::* P> struct Forward {
  typedef Members<P, nullptr> values;
};
template<int Owner::* P> int select() { return 1; }
template<int Owner::* P> int overloaded(int) { return 1; }
template<int* P> int overloaded(long) { return 2; }
int main() {
  return Pointers<nullptr, (int*)nullptr>::count == 2
      && Pointers<>::count == 0
      && Dependent<int, nullptr, (int*)nullptr>::count == 2
      && Members<nullptr, (int Owner::*)nullptr>::count == 2
      && Forward<(int Owner::*)nullptr>::values::count == 2
      && Members<(true ? (int Owner::*)nullptr : nullptr)>::count == 1
      && overloaded<(int Owner::*)nullptr>(0) == 1
      && select<(int Owner::*)nullptr>() == 1 ? 0 : 1;
}
