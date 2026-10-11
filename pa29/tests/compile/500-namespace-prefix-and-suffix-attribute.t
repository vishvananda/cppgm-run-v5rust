#define PRE [[deprecated("prefix")]]
#define SUF __attribute__((__type_visibility__("default")))

namespace PRE both SUF {
struct tag {};
}

int main() {
  return 0;
}
