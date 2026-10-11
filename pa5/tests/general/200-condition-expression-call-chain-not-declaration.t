int a();
int f(int);
int g(int);

int h(int x, int y) {
  if (a && f(x) && g(y))
    return 1;
  return 0;
}
