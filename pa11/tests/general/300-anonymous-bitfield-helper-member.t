typedef unsigned long size_t;

struct S {
  struct {
    size_t a : 1;
    size_t b : sizeof(size_t) * 8 - 1;
  };
  size_t c;
};

int main() {
  return 0;
}
