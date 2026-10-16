// expect exit 0
namespace N {
struct Base {};
struct Token : Base {};
int pick(const Base&) { return 1; }
}
template<class T> int call(T dependent) {
  T first = dependent;
  typedef T Alias;
  Alias second = first;
  auto third = second;
  N::Token fixed;
  using UsingAlias = T;
  UsingAlias from_using;
  return pick(first) * 10000 + pick(second) * 1000 + pick(third) * 100
       + pick(fixed) * 10 + pick(from_using);
}
namespace N { int pick(const Token&) { return 2; } }
int main() {
  N::Token value;
  return call(value) == 22212 ? 0 : 1;
}
