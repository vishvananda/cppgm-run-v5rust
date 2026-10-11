namespace n { typedef int R; struct C {}; }

void f()
{
  n::R (n::C::*p)() = 0;
  (void)p;
}
