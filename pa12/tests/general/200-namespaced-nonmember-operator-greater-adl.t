namespace N {
class D {
public:
  int x;
};

bool operator>(D a, D b) {
  return a.x > b.x;
}
}

int main() {
  N::D a;
  N::D b;
  a.x = 9;
  b.x = 4;
  return a > b ? 0 : 1;
}
