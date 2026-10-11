class Y {
public:
  int y;

  Y() : Y(3) {}
  Y(int x) : y(x) {}
};

int main() {
  Y a;
  return a.y;
}
