// expect exit 0
namespace N {
struct Base {};
struct Token : Base {};
template<class T> int pick(const Base&) { return 1; }
}
using namespace N;
template<class T> int call(N::Token fixed) {
  return pick<T>(fixed) * 10 + pick<int>(fixed);
}
namespace N { template<class T> int pick(const Token&) { return 2; } }
int main() {
  N::Token value;
  return call<char>(value) == 21 ? 0 : 1;
}
