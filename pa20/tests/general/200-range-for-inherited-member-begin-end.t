struct Iter {
  int *p;
  Iter(int *p): p(p) {}
  int operator*() const { return *p; }
  Iter &operator++() { ++p; return *this; }
  bool operator!=(const Iter &o) const { return p != o.p; }
};

struct RangeBase {
  int *data;
  RangeBase(int *data): data(data) {}
  Iter begin() { return Iter(data); }
  Iter end() { return Iter(data + 2); }
};

struct Range : RangeBase {
  Range(int *data): RangeBase(data) {}
};

int main() {
  int data[2] = {5, 6};
  Range r(data);
  int s = 0;
  for (const auto& x : r)
    s = s + x;
  return s;
}
