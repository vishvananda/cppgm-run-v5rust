// VALIDATION: run-pass

struct Hash {};
struct Flavor {};

template<class H, class F, class T>
int test(T const&, unsigned char const (&)[sizeof(T)])
{
  return sizeof(T);
}

int main()
{
  return test<Hash, Flavor>(0, {}) == sizeof(int) ? 0 : 1;
}
