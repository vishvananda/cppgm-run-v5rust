int *& get(int *&p) { return p; }

int f() {
  int x = 1;
  int *p = &x;
  return *get(p);
}
