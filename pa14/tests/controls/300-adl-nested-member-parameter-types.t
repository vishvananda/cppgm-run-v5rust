// expect exit 0
namespace N {
template<class T> struct Outer {
  struct Token {};
  enum Kind { one };
  int call(Token value) { return pick(value); }
  int enumeration(Kind value) { return pick(value); }
};
int pick(Outer<int>::Token) { return 2; }
int pick(Outer<int>::Kind) { return 3; }
}
int main() {
  N::Outer<int> value;
  return value.call(N::Outer<int>::Token()) == 2
      && value.enumeration(N::Outer<int>::one) == 3 ? 0 : 1;
}
