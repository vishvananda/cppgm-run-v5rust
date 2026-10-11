struct Iter {
  int value;

  Iter &operator++(int) {
    value = value + 1;
    return *this;
  }

  Iter &operator*() {
    return *this;
  }

  void set(int v) {
    value = v;
  }
};

int main() {
  Iter it;
  it.value = 3;
  (*it++).set(9);
  return it.value == 9 ? 0 : 1;
}
