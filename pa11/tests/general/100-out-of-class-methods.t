struct YP {
  int x;
  void set(int v);
  int get();
};

void YP::set(int v) { x = v; }
int YP::get() { return x; }

int main() {
  YP p;
  p.set(7);
  return p.get();
}
