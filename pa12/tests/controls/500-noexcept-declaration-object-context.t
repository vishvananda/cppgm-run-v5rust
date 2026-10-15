// expect exit 0
struct S {
  void g() noexcept {}
  void f() noexcept(noexcept(this->g())) { g(); }
  void h() const noexcept(sizeof(*this) > 0) {}
};
int main() { S s; s.f(); s.h(); return 0; }
