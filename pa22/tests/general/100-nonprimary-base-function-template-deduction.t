template<class T>
struct Target {
  int marker;
};

struct Left {
  int left;
};

struct Derived : Left, Target<int> {
  int own;
};

template<class T>
int select(Target<T> &target) {
  target.marker = sizeof(T);
  return target.marker;
}

int main() {
  Derived d;
  d.marker = 0;
  int got = select(d);
  return got == sizeof(int) && d.marker == sizeof(int) ? 0 : 1;
}
