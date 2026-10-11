int gx = 0;

struct S {
  S() {}
  ~S() { gx = gx + 1; }
};

void* operator new(unsigned long, void*) noexcept;

int main() {
  char buf[1];
  S* p = ::new((void*)buf) S();
  typedef S Alias;
  p->~Alias();
  return gx == 1 ? 0 : 1;
}
