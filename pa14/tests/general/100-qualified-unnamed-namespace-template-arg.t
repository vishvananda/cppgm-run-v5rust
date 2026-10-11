template<class T>
struct W {};

namespace ns {
namespace {
struct E {};

int f() {
  W<ns::E const> x;
  return 0;
}
}
}

int main() {
  return ns::f();
}
