struct Handle {
  int* p;

  Handle() : p(0) {}
  Handle(int* x) : p(x) {}

  friend bool operator==(const Handle& a, const Handle& b) {
    return a.p == b.p;
  }

  friend bool operator!=(const Handle& a, const Handle& b) {
    return !(a == b);
  }
};

int main() {
  Handle h;
  return h != 0 ? 1 : 0;
}
