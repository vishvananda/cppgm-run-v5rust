#line 50 "presumed.cpp"
extern int use(int);
volatile int observed;
int f(int x) {
  int y = use(x + 1);
  observed = y;
  (void)use((y));
  if (x) y += use(x);
  for (int i = 0; i < 2; ++i) y += i;
  return y;
}
