struct Box {
  int value;
  Box(int init) : value(init) {}
};

struct Base {
  int make_value() { return 7; }
};

struct Owner : Base {
  int run() {
    Box box(make_value());
    return box.value == 7 ? 0 : 1;
  }
};

int main() {
  Owner owner;
  return owner.run();
}
