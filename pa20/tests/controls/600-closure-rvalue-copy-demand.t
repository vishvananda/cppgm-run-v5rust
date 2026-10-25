// A closure's deleted copy assignment suppresses implicit move members;
// transporting an rvalue closure selects its copy constructor. expect exit 0
template<class F> int invoke(F f) { return f(); }
int main() {
  int value = 7;
  auto empty = [] { return 3; };
  auto ref = [&] { return value; };
  auto first = static_cast<decltype(empty)&&>(empty);
  auto second = static_cast<decltype(ref)&&>(ref);
  return invoke(static_cast<decltype(first)&&>(first))
       + invoke(static_cast<decltype(second)&&>(second)) == 10 ? 0 : 1;
}
