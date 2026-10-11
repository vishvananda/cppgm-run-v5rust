class Y {
public:
  int y;
};

int id(Y y) {
  return y.y;
}

int main() {
  Y a;
  a.y = 8;
  return id(a);
}
