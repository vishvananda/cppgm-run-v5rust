// Deduction cannot convert a closure to match a function-pointer parameter.
struct Sink { template<class T> Sink(T (*)(int)) {} };
int main()
{
  Sink value([](int n) { return n + 1; });
  return 0;
}
