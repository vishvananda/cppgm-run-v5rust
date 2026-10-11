class Inner {
public:
  int *p;
  Inner() : p(0) {}
  Inner(const Inner& other) : p(other.p) {}
};

class Holder {
public:
  Inner inner;
  Inner get() const { return inner; }
};

int main() {
  int x = 7;
  Holder h;
  h.inner.p = &x;
  Inner y = h.get();
  return *y.p - 7;
}
