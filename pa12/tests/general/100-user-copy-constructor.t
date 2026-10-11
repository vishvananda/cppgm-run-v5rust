class Y {
public:
  int y;

  Y() {}

  Y(const Y& r) {
    y = r.y + 1;
  }
};

int main() {
  Y a;
  a.y = 4;
  Y b = a;
  return b.y;
}
