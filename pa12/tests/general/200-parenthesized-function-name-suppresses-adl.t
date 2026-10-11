namespace n {
namespace assoc {

struct A {
  int value;
};

int f(A)
{
  return 2;
}

}

int f(assoc::A)
{
  return 1;
}

namespace inner {

int run()
{
  assoc::A a;
  a.value = 0;
  return (f)(a);
}

}
}

int main()
{
  return n::inner::run() - 1;
}
