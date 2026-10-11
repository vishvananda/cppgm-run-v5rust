// VALIDATION: compile-pass
// N3485 focus: 7.1.5 [dcl.constexpr]

template<class T>
struct identity
{
  typedef T type;
};

constexpr int cast_through_local_typedef(int value)
{
  typedef identity<int>::type local_int;
  typedef local_int second_int;
  return second_int(value);
}

static_assert(cast_through_local_typedef(6) == 6,
              "local typedefs are visible during constexpr evaluation");

int main()
{
  return cast_through_local_typedef(6) == 6 ? 0 : 1;
}
