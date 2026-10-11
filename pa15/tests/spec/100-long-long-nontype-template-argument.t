// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype]

template<long long N>
struct r {};

typedef r<1LL> x;

int main()
{
  return 0;
}
