// VALIDATION: compile-pass

struct Text {
  int tag;

  explicit Text(int) : tag(1) {}
  Text(const char *) : tag(2) {}
};

int main()
{
  Text text = "abc";
  return text.tag == 2 ? 0 : 1;
}
