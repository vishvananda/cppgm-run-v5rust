// Parser work, deferred bodies, source string retention and driver identity.
const char* compiler_author = __CPPGM_AUTHOR__;
namespace observed {
struct Counter {
  Counter(int);
  ~Counter();
  int value;
  int step(int n) {
    for (int i = 0; i < n; ++i) value += i;
    return value;
  }
};
Counter::Counter(int n) : value(n) {}
Counter::~Counter() {}
void syntax(int value) { Counter(value); }
int compute(int n) {
  auto increment = [](int x) { return x + 1; };
  return n ? increment(n) : 0;
}
}
namespace { typedef int word; }
int width = sizeof(observed::Counter(1));
static_assert(true, "source" " observations");
