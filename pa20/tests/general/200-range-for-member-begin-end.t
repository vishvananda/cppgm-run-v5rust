struct Iter {
  int *p;
  Iter(int *p): p(p) {}
  int &operator*() const { return *p; }
  Iter &operator++() { ++p; return *this; }
  bool operator!=(const Iter &o) const { return p != o.p; }
};

struct Range {
  int *data;
  Range(int *data): data(data) {}
  Iter begin() { return Iter(data); }
  Iter end() { return Iter(data + 2); }
};

int main() {
  int data[2] = {1, 2};
  Range r(data);
  int s = 0;
  for (int x : r)
    s = s + x;
  return s;
}
