// N3485 3.4.5p6, 12.4p13, 14.6.1: member qualifier template arguments use
// the expression's scope; explicit destructors retain the selected base and
// unused member bodies stay lazy. expect exit 0
using Choice = int;
int destroyed;
template<class T> struct Base {
  using Choice = char;
  T value;
  ~Base() { ++destroyed; }
  void unused() { typename T::missing value; }
};
struct Derived : Base<int>, Base<char> {};
void destroy_int(Derived* p) { p->Base<Choice>::~Base<int>(); }
int run() {
  Derived d;
  d.Base<Choice>::value = 3;
  d.Base<char>::value = 4;
  return d.Base<int>::value + d.Base<char>::value;
}
int main() { int result = run(); return result == 7 && destroyed == 2 ? 0 : 1; }
