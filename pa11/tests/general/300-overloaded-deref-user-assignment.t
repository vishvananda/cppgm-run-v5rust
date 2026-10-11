int g = 0;

struct Iter {
  Iter& operator=(int n) {
    g = n;
    return *this;
  }

  Iter& operator*() { return *this; }
};

int main() {
  Iter it;
  *it = 7;
  return g - 7;
}
