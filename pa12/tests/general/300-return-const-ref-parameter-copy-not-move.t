struct Box {
  int *p;

  Box(int *value) : p(value) {}
  Box(const Box &other) : p(other.p) {}
  Box(Box &&other) : p(other.p) { other.p = 0; }
};

Box forward_box(const Box &value) {
  return value;
}

int main() {
  int x = 1;
  Box source(&x);
  Box copy = forward_box(source);
  if(source.p != &x) return 1;
  if(copy.p != &x) return 2;
  return 0;
}
