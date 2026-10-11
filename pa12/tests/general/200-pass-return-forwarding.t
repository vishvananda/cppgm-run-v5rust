class Y {
public:
  int y;
};

int get(Y y) {
  return y.y;
}

Y id(Y y) {
  return y;
}

int main() {
  Y a;
  a.y = 6;
  return get(id(a));
}
