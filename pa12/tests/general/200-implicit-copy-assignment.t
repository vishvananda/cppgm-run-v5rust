class Y {
public:
  int y;
};

int main() {
  Y a;
  Y b;
  a.y = 9;
  b = a;
  return b.y;
}
