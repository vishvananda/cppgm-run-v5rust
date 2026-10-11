namespace N {
class D {
public:
  int x;
};

D operator-(D a, D b) {
  D r;
  r.x = a.x - b.x;
  return r;
}

bool operator>(D a, D b) {
  return a.x > b.x;
}
}

int main() {
  N::D a;
  N::D b;
  a.x = 9;
  b.x = 4;
  N::D d = N::operator-(a, b);
  return N::operator>(d, b) ? 0 : 1;
}
