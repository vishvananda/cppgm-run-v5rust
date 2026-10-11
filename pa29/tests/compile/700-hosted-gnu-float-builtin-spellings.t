void f(long double);
void f(__float128);

void g(double);
void g(_Float64);

static_assert(__is_floating_point(__float128), "__float128 is floating");
static_assert(__is_floating_point(_Float64), "_Float64 is floating");

__float128 identity(__float128 value) { return value; }

int main() {
  __float128 value = 1.0F128;
  return identity(value) == value ? 0 : 1;
}
