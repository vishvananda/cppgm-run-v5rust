// expect exit 0
namespace N {
struct Base {};
struct Token : Base {};
int pick(const Base&) { return 1; }
template<class T> struct Box {
  Token value;
  int member() { return pick(value); }
  int self() { return identity(*this); }
};
int pick(const Token&) { return 2; }
int identity(const Box<int>&) { return 3; }
}
int main() {
  N::Box<int> value;
  return value.member() == 2 && value.self() == 3 ? 0 : 1;
}
