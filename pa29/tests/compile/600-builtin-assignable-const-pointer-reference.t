typedef int * const & R;
static const bool value = __is_assignable(R, R);
static_assert(!value, "");

int main() {}
