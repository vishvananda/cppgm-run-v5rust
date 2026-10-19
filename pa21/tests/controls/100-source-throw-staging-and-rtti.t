// expect exit 0
// Throw operands retire their temporaries before the later throw region;
// rethrow stages its unused holder, and a later typeid shares runtime RTTI.
namespace std { struct type_info {}; }
int alive;
struct Token {
  int value;
  Token(int n) : value(n) { ++alive; }
  ~Token() { --alive; }
};
int raise_after_local(bool raise) {
  Token local(3);
  if (raise) throw (Token(7), local.value);
  return local.value;
}
void rethrow_after_local() { Token local(4); throw; }
const std::type_info& query_after_throw() { return typeid(int); }
int main() { return raise_after_local(false) - 3 + alive; }
