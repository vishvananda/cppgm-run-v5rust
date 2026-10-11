class Y {
public:
  int y;
};

Y id(Y y) {
  return y;
}

int main() {
  Y a;
  a.y = 5;
  Y b = id(a);
  return b.y;
}
