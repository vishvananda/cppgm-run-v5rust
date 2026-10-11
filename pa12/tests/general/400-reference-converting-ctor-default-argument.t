struct Alloc {
  int tag;

  Alloc() : tag(5) {}
};

struct Text {
  int marker;

  Text(const Alloc &) : marker(99) {}
  Text(const char *p, const Alloc &a = Alloc()) : marker(p[0] == 'x' ? a.tag : 3) {}
};

int take(const Text &t)
{
  return t.marker == 5 ? 0 : 1;
}

int main()
{
  return take("x");
}
