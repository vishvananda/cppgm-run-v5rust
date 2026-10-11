namespace ns
{
namespace json
{
class value;

namespace detail
{
struct holder
{
  value * data_;

  inline
  ~holder();
};
}

class value
{
public:
  int x;
  ~value() noexcept;
};

namespace detail
{
holder::~holder()
{
  value * p = data_;
  p[0].~value();
}
}

value::~value() noexcept
{
}
}
}
