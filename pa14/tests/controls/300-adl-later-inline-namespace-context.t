// expect exit 0
namespace N {
struct Base {};
struct Token : Base {};
int pick(const Base&) { return 1; }
}
template<class T> int call(T dependent, N::Token fixed) {
  return pick(dependent) * 10 + pick(fixed);
}
namespace N {
inline namespace V { int pick(const Token&) { return 2; } }
}
int main() {
  N::Token value;
  return call(value, value) == 21 ? 0 : 1;
}
