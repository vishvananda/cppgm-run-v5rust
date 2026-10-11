typedef int I;

int f(I& in) {
  I* p = &in;
  I& r = *p;
  return r;
}
