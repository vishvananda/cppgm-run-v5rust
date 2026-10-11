struct Box {
  static const int value = 7;
};

int main() {
  Box box;
  return box.value - 7;
}
