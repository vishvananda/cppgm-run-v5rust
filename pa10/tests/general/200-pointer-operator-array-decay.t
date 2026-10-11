// BSC-113 / BSC-114
int *add_one(int *p) {
  return p + 1;
}

long diff(int *a, int *b) {
  return a - b;
}

int *from_array() {
  int a[4];
  return a + 1;
}

int prefix_ref(int *p) {
  int *&r = ++p;
  return r == p;
}

int array_eq() {
  int a[4];
  return (a + 1) == a;
}

int main() {
  return 0;
}
