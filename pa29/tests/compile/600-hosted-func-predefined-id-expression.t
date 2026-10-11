extern "C" void sink(const char *);

void f()
{
  sink(__func__);
}

int main()
{
  f();
  return 0;
}
