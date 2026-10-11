struct Box {
  int storage[1];

  int operator[](int i) const { return storage[i]; }
};

int main() {
  Box b;
  b.storage[0] = 7;
  return b[0];
}
