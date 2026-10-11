struct Flag {
  int value;

  Flag(int x) : value(x) {}

  explicit operator bool() const {
    return value != 0;
  }
};

int main() {
  Flag a(1);
  Flag b(0);
  if (a && !b)
    return 0;
  return 1;
}
