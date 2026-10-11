struct box {
  int value;

  explicit box(int x) : value(x) {}
};

int read(box b) {
  return b.value;
}

int main() {
  return read(static_cast<box>(7)) - 7;
}
