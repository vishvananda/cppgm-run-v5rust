// HHC-176
namespace n {
int g();
}

int f() {
  using n::g;
  return g();
}
