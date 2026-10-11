// lowir.md elision=copy: a call of a copy or move constructor that is not
// trivial states [elision=copy] when it copy-initializes an object from a
// conditional expression that is a prvalue of the object's class: a
// declaration's `= e` and a by-value argument do; direct-initialization does
// not, and a return's operands construct the result in place (no copy).
struct Token
{
  int id;
  explicit Token(int x) : id(x) {}
  Token(const Token& other) : id(other.id) {}
  Token(Token&& other) : id(other.id) {}
};

int take(Token token) { return token.id; }
Token choose(bool c) { return c ? Token(1) : Token(2); }

int main()
{
  bool c = take(Token(0)) == 0;
  Token copied = c ? Token(3) : Token(4);
  Token direct(c ? Token(5) : Token(6));
  int passed = take(c ? Token(7) : Token(8));
  Token returned = choose(c);
  return copied.id + direct.id + passed + returned.id == 16 ? 0 : 1;
}
