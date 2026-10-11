struct V {
  virtual int z() { return 1; }
  virtual int a() { return 2; }
};

int main() {
  V v;
  return v.z() == 1 && v.a() == 2 ? 0 : 1;
}
