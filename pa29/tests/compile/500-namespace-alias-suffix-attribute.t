#define ATTR __attribute__((__visibility__("default")))

namespace base ATTR {
int f();
}

namespace alias = base;

int main() {
  return 0;
}
