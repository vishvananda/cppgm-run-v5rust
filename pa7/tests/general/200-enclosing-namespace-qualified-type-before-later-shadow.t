namespace outer {
namespace detail {
typedef int value;
}

namespace inner {
detail::value selected = 7;

namespace detail {
typedef long later;
}
}
}

int read()
{
  return outer::inner::selected;
}
