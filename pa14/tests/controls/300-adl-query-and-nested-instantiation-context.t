// expect exit 0
namespace N {
struct Base {};
struct Token : Base {};
int pick(const Base&) { return 1; }
}
using namespace N;
template<class T> struct Wrap { typedef T type; };
template<class T> int call(N::Token fixed) {
  return pick((Wrap<int>(), fixed)) * 100 + pick(fixed) * 10
       + pick((sizeof(T), fixed));
}
namespace N {
int pick(const Token&) { return 2; }
}
int main() {
  N::Token value;
  return call<char>(value) == 111 ? 0 : 1;
}
