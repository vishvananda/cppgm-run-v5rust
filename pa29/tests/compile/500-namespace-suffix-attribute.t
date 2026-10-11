#define ATTR __attribute__((__type_visibility__("default")))

namespace std ATTR {
struct tag {};
}

int main() {
  return 0;
}
