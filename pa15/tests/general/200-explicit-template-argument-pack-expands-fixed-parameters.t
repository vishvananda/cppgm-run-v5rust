template<class A, class B>
char select();

template<class... T>
struct check
{
  static const bool value = sizeof(select<T...>()) == 1;
};

static_assert(check<int, long>::value, "");

int main() {}
