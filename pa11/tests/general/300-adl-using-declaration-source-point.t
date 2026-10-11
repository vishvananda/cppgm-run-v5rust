// ADL sees a function imported into an associated namespace by an earlier
// using-declaration, but namespace collection must not make a later import
// retroactively visible in an already defined function body.

namespace early_model
{
struct value
{
};
}

namespace early_impl
{
bool inspect(early_model::value)
{
  return true;
}
}

namespace early_model
{
using early_impl::inspect;
}

bool early_test()
{
  early_model::value value;
  return inspect(value);
}

namespace late_model
{
struct value
{
};

namespace local
{
bool inspect(value)
{
  return true;
}

bool test()
{
  value item;
  return inspect(item);
}
}
}

namespace late_impl
{
int inspect(late_model::value);
}

namespace late_model
{
using late_impl::inspect;
}

int main()
{
  return early_test() && late_model::local::test() ? 0 : 1;
}
