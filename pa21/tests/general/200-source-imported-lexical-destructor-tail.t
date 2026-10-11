// Imported destructors can throw even when this unit contains no throw/catch.
struct G {
  int id;
  G(int);
  ~G() noexcept(false);
};
extern "C" void f() { G a(1), b(2); }
