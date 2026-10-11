struct Iter {
  int *p;
  Iter(int *p): p(p) {}
  int &operator*() const { return *p; }
  Iter &operator++() { ++p; return *this; }
  bool operator!=(const Iter &o) const { return p != o.p; }
};

namespace ns {
struct Range {
  int *data;
  Range(int *data): data(data) {}
};

Iter begin(Range &r) { return Iter(r.data); }
Iter end(Range &r) { return Iter(r.data + 2); }
}

struct Owner {
  Iter begin();
  int sum(ns::Range &r) {
    int total = 0;
    for (int x : r)
      total = total + x;
    return total;
  }
};

int main() {
  int data[2] = {3, 4};
  ns::Range r(data);
  Owner owner;
  return owner.sum(r);
}
