namespace __attribute__((visibility("default"))) N {
  __attribute__((deprecated)) int f();
  inline __attribute__((visibility("hidden"))) int g();

  template<class T>
  struct X {
    [[nodiscard]] inline __attribute__((visibility("hidden"))) int h() const {
      __asm__("nop");
      return 0;
    }
  };
}

struct rlimit;
int getrlimit(int, struct rlimit*) __asm("_getrlimit");
void abort(void) __attribute__((noreturn));

[[vendor::noreturn]] int ignored_noreturn_attribute;

int main() {
  return 0;
}
