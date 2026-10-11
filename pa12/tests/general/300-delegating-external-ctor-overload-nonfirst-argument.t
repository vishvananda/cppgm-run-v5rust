// VALIDATION: compile-pass

struct Token {
  int value;
};

struct Source {
  const char *c_str() const { return "x"; }
};

struct Box {
  int tag;

  Box(int first, const Token &, int last) : tag(first + last + 10) {}
  Box(int first, const char *, int last);
  Box(const Source &source) : Box(1, source.c_str(), 2) {}
};

int main()
{
  Source source;
  Box box(source);
  return 0;
}
