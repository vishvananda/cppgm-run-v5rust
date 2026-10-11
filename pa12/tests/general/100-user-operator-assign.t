class Y {
public:
  int y;

  Y& operator=(const Y& r) {
    y = r.y + 1;
    return *this;
  }
};

int main() {
  Y a;
  Y b;
  a.y = 4;
  b = a;
  return b.y;
}
