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

int count(D d) {
  return d.x;
}
}

int main() {
  N::D a;
  N::D b;
  a.x = 9;
  b.x = 4;
  return N::count(a - b);
}
