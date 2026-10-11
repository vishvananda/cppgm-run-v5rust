static constexpr float f = 1.0f;
static constexpr double d = 2.0F;
static constexpr long double ld = 3.0L;
static constexpr long double arr[] = {1e0L, 1e1L};

static_assert(f == 1.0f, "");
static_assert(d == 2.0F, "");
static_assert(ld == 3.0L, "");
static_assert(arr[0] == 1.0L, "");
static_assert(arr[1] == 10.0L, "");

int main()
{
  return 0;
}
