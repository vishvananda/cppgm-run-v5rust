// HHC-148
extern "C" void __libcpp_verbose_abort(const char*, ...);

void f() {
  ::__libcpp_verbose_abort("x");
}
