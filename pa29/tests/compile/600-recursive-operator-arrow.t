struct Value {
  int x;
};

struct Proxy {
  Value *p;
  Value *operator->() { return p; }
};

struct Iter {
  Value *p;
  Proxy operator->() { return Proxy{p}; }
};

int main() {
  Value value = {7};
  Iter it = {&value};
  return it->x == 7 ? 0 : 1;
}
