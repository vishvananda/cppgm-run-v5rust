struct A {
  operator int() const volatile {
    return 1;
  }

  operator int() const {
    return 2;
  }
};

int main() {
  return 0;
}
