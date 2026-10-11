#if !__has_extension(is_pod)
#error "missing is_pod extension query"
#endif

#if !__has_extension(is_constructible)
#error "missing is_constructible extension query"
#endif

#if !__has_extension(is_trivially_constructible)
#error "missing is_trivially_constructible extension query"
#endif

template<class T>
struct is_pod
{
  static const bool value = __is_pod(T);
};

struct POD
{
  int POD::*ptr;
};

struct MixedAccess
{
public:
  int a;
private:
  int b;
};

static_assert(__is_trivial(POD), "");
static_assert(__is_standard_layout(POD), "");
static_assert(__is_pod(POD), "");
static_assert(is_pod<POD>::value, "");
static_assert(!__is_standard_layout(MixedAccess), "");

int main()
{
  return 0;
}
