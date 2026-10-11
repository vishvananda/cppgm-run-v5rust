struct YP {
  int x;
  void set(int v) { x = v; }
  int get() { return x; }
};

int main() {
  YP p;
  p.set(7);
  return p.get();
}
