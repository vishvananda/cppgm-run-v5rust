// A mutable static object has a constant address even though reading its
// stored value is not a constant expression.

struct empty_object {};

template<class T>
constexpr T *address_of(T &value)
{
  return &value;
}

static int mutable_scalar = 0;
static empty_object mutable_object = {};

static_assert(address_of(mutable_scalar) == &mutable_scalar,
              "mutable scalar address must remain constant");
static_assert(address_of(mutable_object) == &mutable_object,
              "mutable object address must remain constant");

int main()
{
  return 0;
}
