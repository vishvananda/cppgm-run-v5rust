// VALIDATION: compile-pass

struct String {
  const char *text;

  template<class T>
  String(T p) : text(p) {}
};

struct Stream {
  int tag;

  Stream(int) : tag(1) {}
  Stream(const String &) : tag(2) {}
};

int main()
{
  Stream stream("abc");
  return stream.tag == 2 ? 0 : 1;
}
