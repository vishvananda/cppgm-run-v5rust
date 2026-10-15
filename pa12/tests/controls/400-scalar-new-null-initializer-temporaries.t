// expect exit 0
// Null allocation makes no initializer temporary; a successful one destroys it.
int fail;
int destroyed;
int constructed;
struct Temporary { ~Temporary() { ++destroyed; } };
struct Object {
  Object(const Temporary&) { ++constructed; }
  static void* operator new(unsigned long size) noexcept {
    if (fail) return 0;
    return ::operator new(size);
  }
  static void operator delete(void* p) noexcept { ::operator delete(p); }
};
Object* make() { return new Object(Temporary()); }
void use(const Temporary&, Object* p) { delete p; }
int main() {
  fail = 1;
  Object* p = make();
  if (p || destroyed || constructed) return 1;
  use(Temporary(), new Object(Temporary()));
  if (destroyed != 1 || constructed) return 2;
  fail = 0;
  p = make();
  if (!p || destroyed != 2 || constructed != 1) return 3;
  delete p;
  use(Temporary(), new Object(Temporary()));
  return destroyed == 4 && constructed == 2 ? 0 : 4;
}
