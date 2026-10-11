extern "C" {
typedef decltype(sizeof(int)) size_t;
void *bsearch(const void *key,
              const void *base,
              size_t count,
              size_t width,
              int (* _Nonnull compar)(const void *, const void *));
}

int main() {
  return 0;
}
