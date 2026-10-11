struct Proxy {
  int *p;

  Proxy(int *q) : p(q) {}

  Proxy& operator=(int value) {
    *p = value;
    return *this;
  }
};

struct Box {
  int data[1];

  Proxy operator[](int i) {
    return Proxy(&data[i]);
  }
};

int main() {
  Box b;
  b.data[0] = 0;
  b[0] = 7;
  return b.data[0] - 7;
}
