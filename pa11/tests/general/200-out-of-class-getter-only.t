struct Y {
  int x;
  int get();
};

int Y::get() { return x; }

int main() {
  Y p = {7};
  return p.get() == 7 ? 0 : 1;
}
