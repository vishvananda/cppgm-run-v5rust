// VALIDATION: compile-pass
// N3485 focus: 12.2 [class.temporary]

struct Alloc {
  int value;

  Alloc() : value(3) {}
  ~Alloc() {}
};

struct Text {
  int value;

  Text(const char *, const Alloc &alloc = Alloc()) : value(alloc.value) {}
  Text(const Text &other) : value(other.value) {}
  ~Text() {}
};

Text operator+(const Text &lhs, const Text &)
{
  return Text(lhs.value == 3 ? "sum" : "bad");
}

Text operator+(const Text &lhs, char)
{
  return Text(lhs.value == 3 ? "char" : "bad");
}

int main()
{
  Text delimiter = "";
  char quote = '"';
  Text terminator = Text(")") + delimiter + quote;
  return terminator.value == 3 ? 0 : 1;
}
