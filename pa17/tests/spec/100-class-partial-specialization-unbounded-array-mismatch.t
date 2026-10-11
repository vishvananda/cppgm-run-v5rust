// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec]

enum color
{
  white
};

template<class Y, class T>
struct is_pair_array
{
  enum _vt { value = 1 };
};

template<class Y, class T>
struct is_pair_array<Y[], T[]>
{
  enum _vt { value = 2 + is_pair_array<Y[1], T[1]>::value };
};

static_assert(is_pair_array<color[1], color[1]>::value == 1,
              "known-bound arrays use the primary");
static_assert(is_pair_array<color[], color[]>::value == 3,
              "unknown-bound arrays use the partial");

int main()
{
  return 0;
}
