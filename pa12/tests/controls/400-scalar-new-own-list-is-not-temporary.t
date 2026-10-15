// expect exit 0
// The allocated aggregate is not another initializer temporary or cleanup flag.
struct Aggregate {
  int value;
  ~Aggregate() {}
  static void* operator new(unsigned long size) noexcept {
    return ::operator new(size);
  }
  static void operator delete(void* p) noexcept { ::operator delete(p); }
};
Aggregate* make() { return new Aggregate{7}; }
int main() {
  Aggregate* p = make();
  int value = p->value;
  delete p;
  return value == 7 ? 0 : 1;
}
