struct fobj
{
  int operator()() { return 0; }
  int operator()() const { return 1; }
  int operator()(int x) { return 2 + x; }
  int operator()(int x) const { return 3 + x; }
};

template<class Function>
int call0(Function f)
{
  return f();
}

template<class Function>
int call1(Function f, int x)
{
  return f(x);
}

int main()
{
  fobj f;
  if(call0<fobj>(f) != 0) return 1;
  if(call0<fobj const>(f) != 1) return 2;
  if(call1<fobj>(f, 10) != 12) return 3;
  if(call1<fobj const>(f, 10) != 13) return 4;
  return 0;
}
