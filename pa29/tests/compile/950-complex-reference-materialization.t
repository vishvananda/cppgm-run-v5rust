const _Complex double& pair = __builtin_complex(1.5, 2.5);
const _Complex double& real = 3;
constexpr double component = __real__ static_cast<const _Complex double&>(
    __builtin_complex(4.5, 5.5));

static_assert(__real__ pair == 1.5 && __imag__ pair == 2.5, "pair");
static_assert(__real__ real == 3 && __imag__ real == 0, "converted real");
static_assert(component == 4.5, "reference cast");

double result() {
  return __real__ pair + __imag__ real + component;
}
