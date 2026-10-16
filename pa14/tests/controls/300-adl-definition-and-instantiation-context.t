// expect exit 0
namespace N {
struct Base {};
struct Token : Base {};
int pick(const Base&) { return 1; }
int operator+(const Base&, int) { return 1; }
}
template<class T> int call(T dependent, N::Token fixed) {
  return pick(dependent) * 10 + pick(fixed);
}
template<class T> int operate(T dependent, N::Token fixed) {
  return (dependent + 0) * 10 + (fixed + 0);
}
namespace N {
int pick(const Token&) { return 2; }
int operator+(const Token&, int) { return 2; }
}
int main() {
  N::Token value;
  return call(value, value) == 21 && operate(value, value) == 21 ? 0 : 1;
}
