class Y {
public:
  int y;
};

Y operator-(Y a, Y b) {
  Y r;
  r.y = a.y - b.y;
  return r;
}

int count(Y v) {
  return v.y;
}

int main() {
  Y a;
  Y b;
  a.y = 9;
  b.y = 4;
  return count(a - b);
}
