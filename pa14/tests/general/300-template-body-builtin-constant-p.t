template<class T>
struct builtin_constant_p_body {
  int run(T value) {
    if(__builtin_constant_p(value)) {
      return 1;
    }
    return 0;
  }
};

int main() {
  return 0;
}
