struct Inner {
  int value;

  int & operator[](int) { return value; }
};

struct Outer {
  Inner inner;

  Inner & operator[](int) { return inner; }
};

int main() {
  Outer o;
  o[0][1] = 7;
  return o.inner.value;
}
