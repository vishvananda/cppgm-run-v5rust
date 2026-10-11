class Y {
public:
  int y;
};

int main() {
  Y a;
  a.y = 7;
  Y b = a;
  return b.y;
}
