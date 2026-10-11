// Constructor deduction uses the closure; conversion to a function pointer is later.
template<class T, class U> struct same { static const bool value = false; };
template<class T> struct same<T, T> { static const bool value = true; };
struct Sink
{
  int value;
  template<class F> Sink(F function) : value(function(4))
  { static_assert(!same<F, int (*)(int)>::value, "deduced closure type"); }
};
int main()
{
  Sink captureless = [](int n) { return n + 1; };
  int delta = 1;
  Sink captured = [&delta](int n) { return n + delta; };
  return captureless.value == 5 && captured.value == 5 ? 0 : 1;
}
