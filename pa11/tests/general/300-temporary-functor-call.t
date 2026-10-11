struct F {
  int operator()(int) const {
    return 1;
  }
};

int g(int x) {
  return F()(x);
}

int main() {
  return 0;
}
