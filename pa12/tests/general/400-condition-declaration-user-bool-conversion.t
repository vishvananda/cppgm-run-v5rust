struct Box {
  int p;
  Box() : p(0) {}
  ~Box() {}
  operator bool() const { return p != 0; }
};

static bool f() {
  if(Box direct = Box()) { return false; }
  return true;
}

int main() {
  return f() ? 0 : 10;
}
