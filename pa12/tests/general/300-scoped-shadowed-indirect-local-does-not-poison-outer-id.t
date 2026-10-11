struct S {
  int x;

  S(int v) : x(v) {}
  S(const S& other) : x(other.x) {}
};

int f(int target, int which) {
  switch(which) {
    case 0: {
      S target(7);
      return target.x;
    }
    default:
      return target;
  }
}

int main() {
  return f(5, 1) == 5 ? 0 : 1;
}
