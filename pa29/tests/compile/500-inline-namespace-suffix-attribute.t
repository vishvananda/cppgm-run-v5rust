#define ATTR __attribute__((__visibility__("default")))

namespace outer {
inline namespace abi ATTR {
int f();
}
}

int main() {
  return 0;
}
