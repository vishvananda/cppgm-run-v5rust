extern "C" {
typedef decltype(sizeof(int)) size_t;
void *bsearch_b(const void *key,
                const void *base,
                size_t count,
                size_t width,
                int (^ _Nonnull __compar)(const void *, const void *)
                    __attribute__((__noescape__)));
}

int main() {
  return 0;
}
