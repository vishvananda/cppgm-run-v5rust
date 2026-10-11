struct alloc {
  int tag;
  alloc() : tag(0) {}
  alloc(int seed) : tag(seed) {}
};

struct tag_t {};

struct str {
  int total;

  str() : total(0) {}
  str(tag_t, int first, int second, alloc a) : total(first + second + a.tag) {}

  alloc make_alloc() const { return alloc(4); }

  int build(int first, int second) {
    str temp(tag_t(), first, second, make_alloc());
    return temp.total;
  }
};

int main() {
  str s;
  return s.build(10, 20) == 34 ? 0 : 1;
}
